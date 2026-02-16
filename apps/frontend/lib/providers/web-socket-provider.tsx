'use client'

import type { ReactNode } from 'react'

let lifecycleListenersInstalled = false

type ResumeHandlerRef = { current: (() => void) | null }
type PageFrozenRef = { current: boolean }

function installLifecycleListenersOnce(
  resumeHandlerRef: ResumeHandlerRef,
  pageFrozenRef: PageFrozenRef
) {
  if (lifecycleListenersInstalled || typeof window === 'undefined') return
  lifecycleListenersInstalled = true
  const doc = document
  const win = window

  const tryResume = () => {
    resumeHandlerRef.current?.()
  }

  doc.addEventListener('visibilitychange', () => {
    if (!doc.hidden) tryResume()
  })
  win.addEventListener('pageshow', () => {
    tryResume()
  })
  win.addEventListener('freeze', () => {
    pageFrozenRef.current = true
  })
  win.addEventListener('resume', () => {
    pageFrozenRef.current = false
    tryResume()
  })
  win.addEventListener('focus', () => {
    if (!doc.hidden) tryResume()
  })
  win.addEventListener('online', () => {
    tryResume()
  })
}
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from 'react'
import { useQueryClient } from '@tanstack/react-query'

import type { GameRoomError } from '@/app/game/[gameId]/_components/game-room-view.types'
import {
  resolveWebSocketUrl,
  validateWebSocketConfig,
} from '@/lib/config/env-validation'
import type {
  ClientMsg,
  HelloMsg,
  WireMsg,
} from '@/lib/game-room/protocol/types'
import { isWireMsg } from '@/lib/game-room/protocol/types'
import { logError } from '@/lib/logging/error-logger'
import {
  onLongWaitInvalidated,
  onYourTurn,
  requestLwRefetch,
} from '@/lib/queries/lw-cache'

const MAX_RECONNECT_DELAY_MS = 30_000
const INITIAL_RECONNECT_DELAY_MS = 1000
const WS_TOKEN_FETCH_TIMEOUT_MS = 10_000
const MAX_RECONNECT_ATTEMPTS = 10
const PROTOCOL_VERSION = 1 as const
const HANDSHAKE_TIMEOUT_MS = 10_000

function getReadyStateName(state: number): string {
  switch (state) {
    case WebSocket.CONNECTING:
      return 'CONNECTING'
    case WebSocket.OPEN:
      return 'OPEN'
    case WebSocket.CLOSING:
      return 'CLOSING'
    case WebSocket.CLOSED:
      return 'CLOSED'
    default:
      return `UNKNOWN(${state})`
  }
}

type ConnectionState =
  | 'connecting'
  | 'connected'
  | 'reconnecting'
  | 'disconnected'

interface WebSocketContextType {
  connectionState: ConnectionState
  syncError: GameRoomError | null
  reconnectAttempts: number
  maxReconnectAttempts: number
  sendMessage: (msg: ClientMsg) => void
  registerHandler: (handler: (msg: WireMsg) => void) => () => void
  connect: () => Promise<void>
  disconnect: () => void
}

const WebSocketContext = createContext<WebSocketContextType | null>(null)

type CloseReason = 'manual' | 'unmount' | 'error'

export function WebSocketProvider({
  children,
  isAuthenticated,
}: {
  children: ReactNode
  isAuthenticated: boolean
}) {
  const [connectionState, setConnectionState] =
    useState<ConnectionState>('disconnected')
  const [syncError, setSyncError] = useState<GameRoomError | null>(null)
  const [reconnectAttempts, setReconnectAttempts] = useState(0)

  const queryClient = useQueryClient()

  const wsRef = useRef<WebSocket | null>(null)
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const reconnectDelayRef = useRef(INITIAL_RECONNECT_DELAY_MS)
  const reconnectAttemptsRef = useRef(0)
  const pendingReconnectRef = useRef(false)
  const pageFrozenRef = useRef(false)
  const connectInFlightRef = useRef(false)

  function isPageActive(): boolean {
    if (typeof document === 'undefined') return true
    return !document.hidden && !pageFrozenRef.current
  }
  const hasTriggeredStaleSignoutRef = useRef(false)
  const closeReasonRef = useRef<CloseReason | null>(null)
  const genRef = useRef(0)
  const handlersRef = useRef<Set<(msg: WireMsg) => void>>(new Set())

  // Handshake state
  const helloAckedRef = useRef(false)
  const handshakeTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const clearReconnectTimeout = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current)
      reconnectTimeoutRef.current = null
    }
  }, [])

  const clearHandshakeTimeout = useCallback(() => {
    if (handshakeTimeoutRef.current) {
      clearTimeout(handshakeTimeoutRef.current)
      handshakeTimeoutRef.current = null
    }
  }, [])

  const fetchWsToken = useCallback(async () => {
    const controller = new AbortController()
    const timeoutId = setTimeout(() => {
      controller.abort()
    }, WS_TOKEN_FETCH_TIMEOUT_MS)

    try {
      const response = await fetch('/api/ws-token', {
        method: 'GET',
        cache: 'no-store',
        signal: controller.signal,
      })
      clearTimeout(timeoutId)

      if (!response.ok) {
        if (response.status === 401) {
          if (!hasTriggeredStaleSignoutRef.current) {
            hasTriggeredStaleSignoutRef.current = true
            window.location.href = '/api/auth/signout-session-stale'
          }
          throw new Error('Authentication required')
        }
        throw new Error(
          `Unable to fetch realtime token: ${response.status} ${response.statusText}`
        )
      }

      const body = (await response.json()) as { token?: string }
      if (!body.token) {
        throw new Error('Realtime token missing from response')
      }

      return body.token
    } catch (error) {
      clearTimeout(timeoutId)
      const err = error instanceof Error ? error : new Error(String(error))
      if (err.name === 'AbortError') {
        throw new Error('Request to fetch realtime token timed out')
      }
      throw error
    }
  }, [])

  const resolveWsUrl = useCallback(() => {
    try {
      validateWebSocketConfig()
    } catch (error) {
      logError('WebSocket configuration validation failed', error)
      if (process.env.NODE_ENV === 'development') {
        throw error
      }
    }
    return resolveWebSocketUrl()
  }, [])

  const sendMessage = useCallback((msg: ClientMsg) => {
    const ws = wsRef.current
    if (!ws || ws.readyState !== WebSocket.OPEN) return
    try {
      ws.send(JSON.stringify(msg))
    } catch (error) {
      logError('Failed to send websocket message', error)
    }
  }, [])

  const armHandshakeTimeout = useCallback(
    (ws: WebSocket, myGen: number) => {
      clearHandshakeTimeout()
      handshakeTimeoutRef.current = setTimeout(() => {
        if (genRef.current !== myGen) return
        if (!helloAckedRef.current) {
          closeReasonRef.current = 'error'
          try {
            ws.close(1000, 'Handshake timed out')
          } catch {
            // ignore
          }
          setSyncError({ message: 'Realtime handshake timed out' })
        }
      }, HANDSHAKE_TIMEOUT_MS)
    },
    [clearHandshakeTimeout]
  )

  const cleanupSocketAndTimers = useCallback(
    (reason: CloseReason) => {
      // Invalidate any in-flight handlers / epochs
      genRef.current += 1

      closeReasonRef.current = reason

      clearHandshakeTimeout()
      clearReconnectTimeout()

      const ws = wsRef.current
      wsRef.current = null

      if (ws && ws.readyState === WebSocket.OPEN) {
        try {
          ws.close(1000, 'Closing connection')
        } catch {
          // ignore
        }
      }

      helloAckedRef.current = false
      reconnectAttemptsRef.current = 0
      setReconnectAttempts(0)
    },
    [clearHandshakeTimeout, clearReconnectTimeout]
  )

  const scheduleReconnect = useCallback(
    (connectFn: () => Promise<void>) => {
      if (!isPageActive()) {
        pendingReconnectRef.current = true
        return
      }

      reconnectAttemptsRef.current += 1
      const attempts = reconnectAttemptsRef.current
      setReconnectAttempts(attempts)

      if (attempts > MAX_RECONNECT_ATTEMPTS) {
        setConnectionState('disconnected')
        setSyncError({
          message: `Failed to reconnect after ${MAX_RECONNECT_ATTEMPTS} attempts.`,
        })
        return
      }

      setConnectionState('reconnecting')

      const delay = reconnectDelayRef.current
      const nextDelay = Math.min(
        reconnectDelayRef.current * 2,
        MAX_RECONNECT_DELAY_MS
      )
      reconnectDelayRef.current = nextDelay

      clearReconnectTimeout()
      reconnectTimeoutRef.current = setTimeout(() => {
        if (!isPageActive()) {
          pendingReconnectRef.current = true
          return
        }
        void connectFn()
      }, delay)
    },
    [clearReconnectTimeout]
  )

  const connect = useCallback(async () => {
    if (!isPageActive()) {
      pendingReconnectRef.current = true
      return
    }

    const existingWs = wsRef.current
    if (
      existingWs &&
      (existingWs.readyState === WebSocket.OPEN ||
        existingWs.readyState === WebSocket.CONNECTING)
    ) {
      return
    }

    connectInFlightRef.current = true
    try {
      // State transition: avoid incorrectly flipping 'connecting' -> 'reconnecting'
      setConnectionState((prev) => {
        if (prev === 'connected') return 'connected'
        if (prev === 'reconnecting') return 'reconnecting'
        return prev === 'disconnected' ? 'connecting' : 'connecting'
      })

      const token = await fetchWsToken()
      const wsBase = resolveWsUrl()
      const url = `${wsBase}/ws?token=${encodeURIComponent(token)}`

      genRef.current += 1
      const myGen = genRef.current

      helloAckedRef.current = false

      const ws = new WebSocket(url)
      wsRef.current = ws

      ws.onopen = () => {
        if (genRef.current !== myGen) {
          try {
            ws.close()
          } catch {
            // ignore
          }
          if (wsRef.current === ws) wsRef.current = null
          return
        }

        setSyncError(null)
        armHandshakeTimeout(ws, myGen)

        const helloMsg: HelloMsg = {
          type: 'hello',
          protocol: PROTOCOL_VERSION,
        }
        try {
          ws.send(JSON.stringify(helloMsg))
        } catch (error) {
          logError('Failed to send hello message', error)
        }
      }

      ws.onmessage = (event) => {
        if (genRef.current !== myGen) return
        try {
          const parsed = JSON.parse(event.data) as unknown
          if (!isWireMsg(parsed)) return

          if (parsed.type === 'hello_ack') {
            helloAckedRef.current = true
            clearHandshakeTimeout()

            reconnectDelayRef.current = INITIAL_RECONNECT_DELAY_MS
            reconnectAttemptsRef.current = 0
            setReconnectAttempts(0)

            setConnectionState('connected')
            setSyncError(null)

            // Resync derived caches after reconnect
            void requestLwRefetch(queryClient, { createSnapshot: false })
          }

          if (parsed.type === 'your_turn') {
            const gameId = (parsed as { game_id?: unknown }).game_id
            if (typeof gameId === 'number') {
              void onYourTurn(queryClient, { gameId })
            }
          }

          if (parsed.type === 'long_wait_invalidated') {
            void onLongWaitInvalidated(queryClient)
          }

          handlersRef.current.forEach((handler) => handler(parsed))
        } catch (error) {
          logError('Failed to parse websocket payload', error)
        }
      }

      ws.onerror = () => {
        if (genRef.current !== myGen) return
        if (
          ws.readyState === WebSocket.CLOSING ||
          ws.readyState === WebSocket.CLOSED
        ) {
          return
        }

        logError('WebSocket connection error', new Error('WS Event Error'), {
          readyState: getReadyStateName(ws.readyState),
          url: ws.url?.replace(/token=[^&]+/, 'token=***'),
        })

        setSyncError({ message: 'Websocket connection error' })
      }

      ws.onclose = () => {
        clearHandshakeTimeout()

        if (genRef.current !== myGen) {
          if (wsRef.current === ws) wsRef.current = null
          return
        }

        if (wsRef.current === ws) wsRef.current = null

        const reason = closeReasonRef.current ?? 'error'
        closeReasonRef.current = null
        helloAckedRef.current = false

        if (reason === 'error') {
          scheduleReconnect(connect)
        } else {
          setConnectionState('disconnected')
        }
      }
    } catch (error) {
      logError('Failed to establish realtime connection', error)

      const message =
        error instanceof Error ? error.message : 'Unknown realtime error'
      setSyncError({ message })

      if (
        error instanceof Error &&
        error.message === 'Authentication required'
      ) {
        setConnectionState('disconnected')
        return
      }

      closeReasonRef.current = 'error'
      scheduleReconnect(connect)
    } finally {
      connectInFlightRef.current = false
    }
  }, [
    armHandshakeTimeout,
    clearHandshakeTimeout,
    fetchWsToken,
    queryClient,
    resolveWsUrl,
    scheduleReconnect,
  ])

  const tryResumeReconnect = useCallback(() => {
    if (!isAuthenticated) return
    clearReconnectTimeout()
    if (connectInFlightRef.current) return
    const ws = wsRef.current
    const needsReconnect =
      pendingReconnectRef.current || !ws || ws.readyState !== WebSocket.OPEN
    if (!needsReconnect) return

    pendingReconnectRef.current = false
    reconnectAttemptsRef.current = 0
    setReconnectAttempts(0)
    reconnectDelayRef.current = INITIAL_RECONNECT_DELAY_MS
    void connect()
  }, [isAuthenticated, connect, clearReconnectTimeout])

  const resumeHandlerRef = useRef<(() => void) | null>(null)
  resumeHandlerRef.current = tryResumeReconnect

  const disconnect = useCallback(() => {
    cleanupSocketAndTimers('manual')
    setSyncError(null)
    setConnectionState('disconnected')
  }, [cleanupSocketAndTimers])

  const registerHandler = useCallback((handler: (msg: WireMsg) => void) => {
    handlersRef.current.add(handler)
    return () => {
      handlersRef.current.delete(handler)
    }
  }, [])

  useEffect(() => {
    if (typeof window !== 'undefined')
      installLifecycleListenersOnce(resumeHandlerRef, pageFrozenRef)
  }, [])

  useEffect(() => {
    if (isAuthenticated) {
      void connect()
    } else {
      disconnect()
    }

    return () => {
      // On provider unmount, clean up socket/timers.
      cleanupSocketAndTimers('unmount')
    }
  }, [isAuthenticated, connect, disconnect, cleanupSocketAndTimers])

  return (
    <WebSocketContext.Provider
      value={{
        connectionState,
        syncError,
        reconnectAttempts,
        maxReconnectAttempts: MAX_RECONNECT_ATTEMPTS,
        sendMessage,
        registerHandler,
        connect,
        disconnect,
      }}
    >
      {children}
    </WebSocketContext.Provider>
  )
}

export function useWebSocket() {
  const context = useContext(WebSocketContext)
  if (!context) {
    throw new Error('useWebSocket must be used within a WebSocketProvider')
  }
  return context
}
