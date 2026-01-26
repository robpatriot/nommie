import { useCallback, useEffect, useRef, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'

import {
  resolveWebSocketUrl,
  validateWebSocketConfig,
} from '@/lib/config/env-validation'
import { logError } from '@/lib/logging/error-logger'

import {
  getGameRoomSnapshotAction,
  type GameRoomSnapshotPayload,
} from '@/app/actions/game-room-actions'
import type { GameRoomError } from '@/app/game/[gameId]/_components/game-room-view.types'
import type {
  GameStateMsg,
  HelloMsg,
  SubscribeMsg,
  WireMsg,
} from '@/lib/game-room/protocol/types'
import { isGameStateMsg, isWireMsg } from '@/lib/game-room/protocol/types'
import { queryKeys } from '@/lib/queries/query-keys'
import { gameStateMsgToSnapshotPayload } from '@/lib/game-room/protocol/transform'

// Helper to get readable WebSocket readyState names
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

export interface UseGameSyncResult {
  refreshSnapshot: () => Promise<void>
  connectionState: ConnectionState
  syncError: GameRoomError | null
  isRefreshing: boolean
  disconnect: () => void
  connect: () => Promise<void>
  reconnectAttempts: number
  maxReconnectAttempts: number
}

interface UseGameSyncOptions {
  initialData: GameRoomSnapshotPayload
  gameId: number
}

const MAX_RECONNECT_DELAY_MS = 30_000
const INITIAL_RECONNECT_DELAY_MS = 1000
const WS_TOKEN_FETCH_TIMEOUT_MS = 10_000
const MAX_RECONNECT_ATTEMPTS = 10

// Backend-locked transport protocol version for hello/ack (not the game_state version)
const PROTOCOL_VERSION = 1 as const

// Handshake stall guard. Long enough for prod variance; keeps tests from hanging forever.
// (If you want per-env tuning later, make this configurable.)
const HANDSHAKE_TIMEOUT_MS = 10_000

/**
 * Reasons for closing a connection - used to determine reconnection policy
 */
type CloseReason =
  | 'manual' // User explicitly disconnected
  | 'gameIdChange' // Game ID changed, new connection will be created
  | 'unmount' // Component unmounting
  | 'replace' // Replacing existing connection
  | 'error' // Connection error - should attempt reconnection

/**
 * Centralized cleanup function - clears timeouts and closes connection
 * Note: Does NOT null handlers - let onclose fire naturally so it can
 * make informed decisions based on context (closeReasonRef)
 */
function cleanupWebSocket(
  ws: WebSocket | null,
  reconnectTimeout: ReturnType<typeof setTimeout> | null
): void {
  if (reconnectTimeout) {
    clearTimeout(reconnectTimeout)
  }
  if (ws) {
    // Only close OPEN sockets - don't close CONNECTING sockets to avoid browser errors
    // CONNECTING sockets will either:
    // 1. Connect and then onopen handler will see generation mismatch and close it
    // 2. Fail naturally and onerror/onclose handlers will see generation mismatch and ignore
    // The generation token ensures stale connections don't affect state
    if (ws.readyState === WebSocket.OPEN) {
      // Close the connection - onclose will fire naturally
      // The handler will check closeReasonRef to decide if reconnection is needed
      // We don't null handlers here - that would prevent onclose from firing
      ws.close(1000, 'Connection closed')
    }
    // For CONNECTING, CLOSING, or CLOSED states, let handlers clean up via generation checks
  }
}

export function useGameSync({
  initialData,
  gameId,
}: UseGameSyncOptions): UseGameSyncResult {
  const queryClient = useQueryClient()
  const [connectionState, setConnectionState] =
    useState<ConnectionState>('connecting')
  const [syncError, setSyncError] = useState<GameRoomError | null>(null)
  const [isRefreshing, setIsRefreshing] = useState(false)
  const [reconnectAttempts, setReconnectAttempts] = useState(0)

  // All mutable state in refs for stable function references
  const wsRef = useRef<WebSocket | null>(null)
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const reconnectDelayRef = useRef(INITIAL_RECONNECT_DELAY_MS)
  const reconnectAttemptsRef = useRef(0)
  const closeReasonRef = useRef<CloseReason | null>(null)
  const etagRef = useRef<string | undefined>(initialData.etag)

  // Generation token: increments on each connection attempt and cleanup
  // Stale handlers from previous effect runs check this and bail early
  const genRef = useRef(0)
  const gameIdRef = useRef(gameId)

  // WS transport-level monotonic version
  const lastWsVersionSeenRef = useRef<number | undefined>(undefined)

  // Handshake state (per active socket connection attempt)
  const helloAckedRef = useRef(false)
  const subscribedRef = useRef(false)
  const handshakeTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Message handler ref - updated on every render to always have latest
  const handleMessageEventRef = useRef<
    ((event: MessageEvent<string>) => void) | undefined
  >(undefined)

  const buildEtag = useCallback(
    (version?: number) =>
      typeof version === 'number'
        ? `"game-${gameIdRef.current}-v${version}"`
        : undefined,
    []
  )

  const applySnapshot = useCallback(
    (payload: GameRoomSnapshotPayload) => {
      const currentGameId = gameIdRef.current
      // Defensive check: ignore older snapshots to prevent out-of-order updates
      const current = queryClient.getQueryData<GameRoomSnapshotPayload>(
        queryKeys.games.snapshot(currentGameId)
      )
      if (
        current &&
        current.version !== undefined &&
        payload.version !== undefined &&
        current.version >= payload.version
      ) {
        return
      }

      etagRef.current = payload.etag ?? buildEtag(payload.version)
      setSyncError(null)
      queryClient.setQueryData(queryKeys.games.snapshot(currentGameId), payload)

      // Keep "waitingLongest" fresh when any snapshot arrives (required)
      queryClient.invalidateQueries({
        queryKey: queryKeys.games.waitingLongest(),
      })
    },
    [buildEtag, queryClient]
  )

  const refreshSnapshot = useCallback(async () => {
    setIsRefreshing(true)
    try {
      const result = await getGameRoomSnapshotAction({
        gameId: gameIdRef.current,
        etag: etagRef.current,
      })

      if (result.kind === 'ok') {
        applySnapshot(result.data)
      } else if (result.kind === 'not_modified') {
        const cachedData = queryClient.getQueryData<GameRoomSnapshotPayload>(
          queryKeys.games.snapshot(gameIdRef.current)
        )
        if (cachedData) {
          queryClient.setQueryData(
            queryKeys.games.snapshot(gameIdRef.current),
            {
              ...cachedData,
              timestamp: new Date().toISOString(),
            }
          )
        }
      } else {
        setSyncError({ message: result.message, traceId: result.traceId })
      }
    } catch (error) {
      logError('Manual snapshot refresh failed', error, {
        gameId: gameIdRef.current,
      })
      setSyncError({
        message:
          error instanceof Error
            ? error.message
            : 'Unable to refresh game state',
      })
    } finally {
      setIsRefreshing(false)
    }
  }, [applySnapshot, queryClient])

  const handleGameStateMessage = useCallback(
    (message: GameStateMsg) => {
      // Only apply updates for the currently viewed game.
      if (
        message.topic?.kind !== 'game' ||
        message.topic.id !== gameIdRef.current
      ) {
        return
      }

      // Debug/observability only: record last WS version seen for this game.
      lastWsVersionSeenRef.current = message.version

      const payload = gameStateMsgToSnapshotPayload(message)
      applySnapshot(payload)
    },
    [applySnapshot]
  )

  const sendJson = useCallback((msg: unknown) => {
    const ws = wsRef.current
    if (!ws || ws.readyState !== WebSocket.OPEN) return
    try {
      ws.send(JSON.stringify(msg))
    } catch (error) {
      logError('Failed to send websocket message', error, {
        gameId: gameIdRef.current,
      })
    }
  }, [])

  const clearHandshakeTimeout = useCallback(() => {
    if (handshakeTimeoutRef.current) {
      clearTimeout(handshakeTimeoutRef.current)
      handshakeTimeoutRef.current = null
    }
  }, [])

  const armHandshakeTimeout = useCallback(
    (ws: WebSocket, myGen: number) => {
      clearHandshakeTimeout()
      handshakeTimeoutRef.current = setTimeout(() => {
        // Ignore if stale
        if (genRef.current !== myGen) return
        // If still not subscribed, treat as error and let reconnect policy handle it
        if (!subscribedRef.current) {
          closeReasonRef.current = 'error'
          try {
            ws.close(1000, 'Handshake timed out')
          } catch {
            // ignore
          }
          setSyncError({
            message: 'Realtime handshake timed out',
            traceId: undefined,
          })
        }
      }, HANDSHAKE_TIMEOUT_MS)
    },
    [clearHandshakeTimeout]
  )

  // Handle incoming WS messages (game_state + handshake envelopes + keep other message types safe)
  const handleMessageEvent = useCallback(
    (event: MessageEvent<string>) => {
      try {
        const parsed = JSON.parse(event.data) as unknown

        if (!isWireMsg(parsed)) return

        switch ((parsed as { type: unknown }).type) {
          case 'hello_ack': {
            // Handshake step: hello_ack -> subscribe(current game)
            helloAckedRef.current = true
            subscribedRef.current = false
            sendJson({
              type: 'subscribe',
              topic: { kind: 'game', id: gameIdRef.current },
            } satisfies SubscribeMsg)
            return
          }

          case 'ack': {
            // Any ack after hello_ack is treated as successful subscription.
            if (!subscribedRef.current && helloAckedRef.current) {
              subscribedRef.current = true
              clearHandshakeTimeout()

              // Reset reconnection state on successful handshake (not merely onopen)
              reconnectDelayRef.current = INITIAL_RECONNECT_DELAY_MS
              reconnectAttemptsRef.current = 0
              setReconnectAttempts(0)

              setConnectionState('connected')
              setSyncError(null)
            }
            return
          }

          case 'game_state': {
            if (isGameStateMsg(parsed)) {
              handleGameStateMessage(parsed)
            }
            return
          }

          default: {
            // Must not break other message types.
            // yourturn: keep waitingLongest fresh.
            if (
              (parsed as WireMsg).type === 'yourturn' ||
              (parsed as WireMsg).type === 'your_turn'
            ) {
              queryClient.invalidateQueries({
                queryKey: queryKeys.games.waitingLongest(),
              })
              return
            }

            // error: preserve original behavior (HTTP refresh fallback)
            if ((parsed as WireMsg).type === 'error') {
              void refreshSnapshot()
              return
            }

            return
          }
        }
      } catch (error) {
        logError('Failed to parse websocket payload', error, {
          gameId: gameIdRef.current,
        })
      }
    },
    [
      clearHandshakeTimeout,
      handleGameStateMessage,
      queryClient,
      refreshSnapshot,
      sendJson,
    ]
  )

  // Keep ref in sync with latest handler (preserve original behavior)
  handleMessageEventRef.current = handleMessageEvent

  /**
   * Determines if reconnection should be attempted based on close reason
   */
  const shouldReconnect = useCallback((reason: CloseReason | null): boolean => {
    // Only reconnect on errors - all other reasons are intentional closes
    return reason === 'error'
  }, [])

  /**
   * Schedules a reconnection attempt with exponential backoff
   * Stops attempting after MAX_RECONNECT_ATTEMPTS
   */
  const scheduleReconnect = useCallback((connectFn: () => Promise<void>) => {
    reconnectAttemptsRef.current += 1
    const currentAttempts = reconnectAttemptsRef.current
    setReconnectAttempts(currentAttempts)

    if (currentAttempts > MAX_RECONNECT_ATTEMPTS) {
      // Max attempts reached - give up and show error
      setConnectionState('disconnected')
      setSyncError({
        message: `Failed to reconnect after ${MAX_RECONNECT_ATTEMPTS} attempts. Please refresh the page.`,
        traceId: undefined,
      })
      logError(
        'WebSocket max reconnection attempts reached',
        new Error('Max reconnection attempts exceeded'),
        {
          gameId: gameIdRef.current,
          attempts: currentAttempts,
        }
      )
      return
    }

    setConnectionState('reconnecting')
    const delay = reconnectDelayRef.current
    reconnectDelayRef.current = Math.min(
      reconnectDelayRef.current * 2,
      MAX_RECONNECT_DELAY_MS
    )

    reconnectTimeoutRef.current = setTimeout(() => {
      void connectFn()
    }, delay)
  }, [])

  // Store connect function in ref to avoid circular dependency
  const connectRef = useRef<(() => Promise<void>) | null>(null)

  const fetchWsToken = useCallback(async () => {
    const controller = new AbortController()
    const timeoutId = setTimeout(
      () => controller.abort(),
      WS_TOKEN_FETCH_TIMEOUT_MS
    )

    try {
      const response = await fetch('/api/ws-token', {
        method: 'GET',
        cache: 'no-store',
        signal: controller.signal,
      })
      clearTimeout(timeoutId)

      if (!response.ok) {
        if (response.status === 401) {
          // Session is invalid - hard redirect to home/login
          // This breaks the retry loop effectively
          window.location.href = '/'
          await new Promise((resolve) => setTimeout(resolve, 10000)) // Stall while redirecting
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
      if (error instanceof Error && error.name === 'AbortError') {
        throw new Error('Request to fetch realtime token timed out')
      }
      throw error
    }
  }, [])

  const resolveWsUrl = useCallback(() => {
    try {
      validateWebSocketConfig()
    } catch (error) {
      logError('WebSocket configuration validation failed', error, {
        gameId: gameIdRef.current,
      })
      if (process.env.NODE_ENV === 'development') {
        throw error
      }
    }
    return resolveWebSocketUrl()
  }, [])

  /**
   * Closes existing connection with a specific reason
   * The reason determines whether reconnection will be attempted
   * Note: The reason is set before close() and will be read by onclose handler
   */
  const closeExistingConnection = useCallback(
    (reason: CloseReason) => {
      if (wsRef.current) {
        const existingWs = wsRef.current
        // Set close reason before closing - onclose handler will read and reset it
        closeReasonRef.current = reason
        cleanupWebSocket(existingWs, null)
        // Let onclose handler clear wsRef.current via its check
      }
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current)
        reconnectTimeoutRef.current = null
      }
      clearHandshakeTimeout()
    },
    [clearHandshakeTimeout]
  )

  const connect = useCallback(async () => {
    // Check if we already have an active connection
    const existingWs = wsRef.current
    if (
      existingWs &&
      (existingWs.readyState === WebSocket.OPEN ||
        existingWs.readyState === WebSocket.CONNECTING)
    ) {
      return
    }

    try {
      // Reset reconnection attempts on manual connect (user-initiated retry)
      reconnectAttemptsRef.current = 0
      setReconnectAttempts(0)
      reconnectDelayRef.current = INITIAL_RECONNECT_DELAY_MS

      // Close any existing connection - mark as 'replace' since we're creating a new one
      closeExistingConnection('replace')

      setConnectionState((state) =>
        state === 'disconnected' ? 'reconnecting' : 'connecting'
      )

      const token = await fetchWsToken()

      const wsBase = resolveWsUrl()
      // Locked by you: token from /api/ws-token AND new URL shape /ws?token=...
      const url = `${wsBase}/ws?token=${encodeURIComponent(token)}`

      // Increment generation and capture it - this invalidates any previous connection attempts
      genRef.current++
      const myGen = genRef.current

      // Reset handshake state for this connection attempt
      helloAckedRef.current = false
      subscribedRef.current = false

      const ws = new WebSocket(url)
      wsRef.current = ws

      ws.onopen = () => {
        // Generation check: if this handler is from a stale connection, ignore it
        if (genRef.current !== myGen) {
          ws.close()
          // Clear ref if it still points to this stale socket
          if (wsRef.current === ws) {
            wsRef.current = null
          }
          return
        }

        setConnectionState('connecting')
        setSyncError(null)

        // Start handshake timeout guard
        armHandshakeTimeout(ws, myGen)

        // Handshake: say hello
        sendJson({
          type: 'hello',
          protocol: PROTOCOL_VERSION,
        } satisfies HelloMsg)
      }

      ws.onmessage = (event) => {
        // Generation check: ignore messages from stale connections
        if (genRef.current !== myGen) {
          return
        }
        handleMessageEventRef.current?.(event)
      }

      ws.onerror = (event) => {
        // Generation check: ignore errors from stale connections
        if (genRef.current !== myGen) {
          return
        }

        const wsUrl = ws.url?.replace(/token=[^&]+/, 'token=***') || 'unknown'
        const wsReadyState = ws.readyState
        const wsReadyStateName = getReadyStateName(wsReadyState)

        // Only log errors if the WebSocket is not already closing/closed
        if (
          wsReadyState === WebSocket.CLOSING ||
          wsReadyState === WebSocket.CLOSED
        ) {
          return
        }

        const errorDetails: Record<string, unknown> = {
          gameId: gameIdRef.current,
          url: wsUrl,
          readyState: wsReadyState,
          readyStateName: wsReadyStateName,
          eventType: event.type || 'error',
          timestamp: new Date().toISOString(),
        }

        if (event.target) {
          const target = event.target as WebSocket
          errorDetails.eventTarget = {
            readyState: target.readyState,
            readyStateName: getReadyStateName(target.readyState),
            url: target.url?.replace(/token=[^&]+/, 'token=***') || 'unknown',
            protocol: target.protocol || 'none',
            extensions: target.extensions || 'none',
          }
        }

        const errorMessage = `WebSocket connection error: gameId=${gameIdRef.current}, readyState=${wsReadyStateName}, url=${wsUrl}`
        logError(errorMessage, new Error(errorMessage), errorDetails)

        setSyncError({
          message: 'Websocket connection error',
          traceId: undefined,
        })
      }

      // Thin event handler - just notifies owner, doesn't make policy decisions
      ws.onclose = (_event) => {
        // Clear handshake timeout regardless
        clearHandshakeTimeout()

        // Generation check: ignore close events from stale connections
        // But still clear the ref if it points to this socket to avoid leaks
        if (genRef.current !== myGen) {
          if (wsRef.current === ws) {
            wsRef.current = null
          }
          return
        }

        // If this is still the current connection, clear it
        if (wsRef.current === ws) {
          wsRef.current = null
        }

        // Get the close reason (set by closeExistingConnection or default to error)
        const reason = closeReasonRef.current ?? 'error'
        closeReasonRef.current = null

        // Reset handshake state
        helloAckedRef.current = false
        subscribedRef.current = false

        // Owner makes policy decision based on context
        if (shouldReconnect(reason)) {
          const connectFn = connectRef.current
          if (connectFn) {
            scheduleReconnect(connectFn)
          }
        } else {
          setConnectionState('disconnected')
        }
      }
    } catch (error) {
      const currentGameId = gameIdRef.current
      const normalizedError =
        error instanceof Error
          ? error
          : new Error(
              typeof error === 'object' && error !== null && 'message' in error
                ? String((error as { message?: unknown }).message)
                : 'Failed to establish realtime connection'
            )
      logError('Failed to establish realtime connection', normalizedError, {
        gameId: currentGameId,
        originalErrorType:
          error instanceof Error ? error.constructor.name : typeof error,
        originalErrorMessage:
          error instanceof Error ? error.message : String(error),
      })
      setSyncError({
        message: normalizedError.message,
      })
      // Connection failed to establish - treat as error and attempt reconnect
      const connectFn = connectRef.current
      if (connectFn) {
        scheduleReconnect(connectFn)
      }
    }
  }, [
    armHandshakeTimeout,
    clearHandshakeTimeout,
    closeExistingConnection,
    fetchWsToken,
    resolveWsUrl,
    scheduleReconnect,
    sendJson,
    shouldReconnect,
  ])

  // Store connect function in ref for use in onclose handler
  connectRef.current = connect

  const disconnect = useCallback(() => {
    // Increment generation to invalidate any in-flight connection
    genRef.current++
    // Reset reconnection attempts on manual disconnect
    reconnectAttemptsRef.current = 0
    setReconnectAttempts(0)
    // Set close reason to 'manual' - onclose handler will read and reset it
    closeReasonRef.current = 'manual'
    clearHandshakeTimeout()
    const ws = wsRef.current
    cleanupWebSocket(ws, reconnectTimeoutRef.current)
    // Let onclose handler clear wsRef.current via its check
    reconnectTimeoutRef.current = null
  }, [clearHandshakeTimeout])

  const beginNewConnectionEpoch = useCallback(() => {
    // Invalidate any in-flight handlers or connection attempts
    genRef.current += 1

    // Reset handshake gates for the new subscription epoch
    helloAckedRef.current = false
    subscribedRef.current = false

    // Reset reconnection/backoff state
    reconnectAttemptsRef.current = 0
    setReconnectAttempts(0)
    reconnectDelayRef.current = INITIAL_RECONNECT_DELAY_MS

    // Clear any previous error so the new game starts clean
    setSyncError(null)

    // Intentionally close the existing socket; must NOT trigger reconnect
    closeExistingConnection('gameIdChange')
  }, [closeExistingConnection])

  // Main effect: manage connection lifecycle based on gameId.
  // A change in gameId defines a new *connection epoch*.
  useEffect(() => {
    const previousGameId = gameIdRef.current
    const gameIdChanged = previousGameId !== gameId
    gameIdRef.current = gameId

    const closeReason: CloseReason = gameIdChanged ? 'gameIdChange' : 'unmount'

    if (gameIdChanged) {
      beginNewConnectionEpoch()
      void connect()
      return
    }

    void connect()

    return () => {
      genRef.current += 1
      closeReasonRef.current = closeReason

      const ws = wsRef.current
      if (ws) {
        cleanupWebSocket(ws, reconnectTimeoutRef.current)
        wsRef.current = null
      }

      reconnectTimeoutRef.current = null
      clearHandshakeTimeout()
    }
  }, [gameId, beginNewConnectionEpoch, clearHandshakeTimeout, connect])

  useEffect(() => {
    etagRef.current = initialData.etag
  }, [initialData.etag])

  return {
    refreshSnapshot,
    connectionState,
    syncError,
    isRefreshing,
    disconnect,
    connect,
    reconnectAttempts,
    maxReconnectAttempts: MAX_RECONNECT_ATTEMPTS,
  }
}
