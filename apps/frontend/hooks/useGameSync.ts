import { useCallback, useEffect, useRef, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import {
  resolveWebSocketUrl,
  validateWebSocketConfig,
} from '@/lib/config/env-validation'
import { logError } from '@/lib/logging/error-logger'

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

import {
  getGameRoomSnapshotAction,
  type GameRoomSnapshotPayload,
} from '@/app/actions/game-room-actions'
import type { GameRoomError } from '@/app/game/[gameId]/_components/game-room-view.types'
import type { BidConstraints, GameSnapshot, Seat } from '@/lib/game-room/types'
import { extractPlayerNames } from '@/utils/player-names'
import { queryKeys } from '@/lib/queries/query-keys'

type ConnectionState =
  | 'connecting'
  | 'connected'
  | 'reconnecting'
  | 'disconnected'

interface SnapshotEnvelopeFromWs {
  snapshot: GameSnapshot
  viewer_hand?: string[] | null
  bid_constraints?: {
    zero_bid_locked?: boolean
  } | null
  version: number
}

interface SnapshotMessage {
  type: 'snapshot'
  data: SnapshotEnvelopeFromWs
  viewer_seat?: number | null
}

interface ErrorMessage {
  type: 'error'
  code?: string
  message: string
}

type WsMessage = SnapshotMessage | ErrorMessage | Record<string, unknown>

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
        current.version > payload.version
      ) {
        return
      }

      etagRef.current = payload.etag ?? buildEtag(payload.version)
      setSyncError(null)
      queryClient.setQueryData(queryKeys.games.snapshot(currentGameId), payload)
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

  const transformSnapshotMessage = useCallback(
    (message: SnapshotMessage): GameRoomSnapshotPayload => {
      const { data, viewer_seat } = message
      const version = data.version
      const viewerSeat =
        typeof viewer_seat === 'number' && viewer_seat >= 0 && viewer_seat <= 3
          ? (viewer_seat as Seat)
          : null

      const bidConstraints: BidConstraints | null = data.bid_constraints
        ? {
            zeroBidLocked: Boolean(data.bid_constraints.zero_bid_locked),
          }
        : null

      const normalizedViewerHand = Array.isArray(data.viewer_hand)
        ? data.viewer_hand
        : []

      const playerNames = extractPlayerNames(data.snapshot.game.seating)

      return {
        snapshot: data.snapshot,
        playerNames,
        viewerSeat,
        viewerHand: normalizedViewerHand,
        timestamp: new Date().toISOString(),
        hostSeat: data.snapshot.game.host_seat as Seat,
        bidConstraints,
        version,
        etag: buildEtag(version),
      }
    },
    [buildEtag]
  )

  const handleSnapshotMessage = useCallback(
    (message: SnapshotMessage) => {
      const payload = transformSnapshotMessage(message)
      applySnapshot(payload)
    },
    [applySnapshot, transformSnapshotMessage]
  )

  const handleMessageEvent = useCallback(
    (event: MessageEvent<string>) => {
      try {
        const parsed = JSON.parse(event.data) as WsMessage
        if (parsed.type === 'snapshot' && 'data' in parsed) {
          handleSnapshotMessage(parsed as SnapshotMessage)
          return
        }

        if (parsed.type === 'error' && 'message' in parsed) {
          void refreshSnapshot()
        }
      } catch (error) {
        logError('Failed to parse websocket payload', error, {
          gameId: gameIdRef.current,
        })
      }
    },
    [handleSnapshotMessage, refreshSnapshot]
  )

  // Keep ref in sync with latest handler
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
  const closeExistingConnection = useCallback((reason: CloseReason) => {
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
  }, [])

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

      const currentGameId = gameIdRef.current

      const token = await fetchWsToken()

      const wsBase = resolveWsUrl()
      const url = `${wsBase}/ws/games/${currentGameId}?token=${encodeURIComponent(token)}`

      // Increment generation and capture it - this invalidates any previous connection attempts
      genRef.current++
      const myGen = genRef.current

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

        // Reset reconnection state on successful connection
        reconnectDelayRef.current = INITIAL_RECONNECT_DELAY_MS
        reconnectAttemptsRef.current = 0
        setConnectionState('connected')
        setSyncError(null) // Clear any previous connection errors
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
        // Errors during close are expected and not actionable
        if (
          wsReadyState === WebSocket.CLOSING ||
          wsReadyState === WebSocket.CLOSED
        ) {
          return
        }

        // This is a real connection error - capture details
        const errorDetails: Record<string, unknown> = {
          gameId: currentGameId,
          url: wsUrl,
          readyState: wsReadyState,
          readyStateName: wsReadyStateName,
          eventType: event.type || 'error',
          timestamp: new Date().toISOString(),
        }

        // Capture event target details if available
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

        const errorMessage = `WebSocket connection error: gameId=${currentGameId}, readyState=${wsReadyStateName}, url=${wsUrl}`

        logError(errorMessage, new Error(errorMessage), errorDetails)

        setSyncError({
          message: 'Websocket connection error',
          traceId: undefined,
        })
      }

      // Thin event handler - just notifies owner, doesn't make policy decisions
      ws.onclose = (_event) => {
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
        // Read and reset atomically
        const reason = closeReasonRef.current ?? 'error'
        closeReasonRef.current = null

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
                ? String(error.message)
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
    fetchWsToken,
    resolveWsUrl,
    scheduleReconnect,
    closeExistingConnection,
    shouldReconnect,
    // Note: connectionState is intentionally NOT in deps - we use it only for logging
    // and don't want to recreate connect when it changes
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
    const ws = wsRef.current
    cleanupWebSocket(ws, reconnectTimeoutRef.current)
    // Let onclose handler clear wsRef.current via its check
    reconnectTimeoutRef.current = null
  }, [])

  // Main effect: manage connection lifecycle based on gameId
  useEffect(() => {
    // Update ref immediately (synchronous)
    const previousGameId = gameIdRef.current
    const gameIdChanged = previousGameId !== gameId
    gameIdRef.current = gameId

    // Determine close reason based on context
    const closeReason: CloseReason = gameIdChanged ? 'gameIdChange' : 'unmount'

    // Reset reconnection attempts when gameId changes (new connection)
    if (gameIdChanged) {
      reconnectAttemptsRef.current = 0
      setReconnectAttempts(0)
      reconnectDelayRef.current = INITIAL_RECONNECT_DELAY_MS

      // Close existing connection if gameId changed
      const existingWs = wsRef.current
      if (existingWs) {
        closeReasonRef.current = 'gameIdChange'
        cleanupWebSocket(existingWs, reconnectTimeoutRef.current)
      }
    }

    // Simply call connect() - it will check for existing connections internally
    void connect()

    return () => {
      // Increment generation to invalidate any in-flight connection from this effect run
      // Note: We intentionally mutate genRef.current here - we want to increment it, not copy it
      // eslint-disable-next-line react-hooks/exhaustive-deps
      genRef.current++

      // Set close reason - onclose handler will read and reset it
      closeReasonRef.current = closeReason

      // Close and clear the socket
      const ws = wsRef.current
      if (ws) {
        cleanupWebSocket(ws, reconnectTimeoutRef.current)
        wsRef.current = null
      }

      // Clear reconnect timer
      reconnectTimeoutRef.current = null
    }
  }, [gameId, connect])

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
