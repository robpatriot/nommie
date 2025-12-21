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
  lock_version: number
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
}

interface UseGameSyncOptions {
  initialData: GameRoomSnapshotPayload
  gameId: number
}

const MAX_RECONNECT_DELAY_MS = 30_000
const INITIAL_RECONNECT_DELAY_MS = 1000

export function useGameSync({
  initialData,
  gameId,
}: UseGameSyncOptions): UseGameSyncResult {
  const queryClient = useQueryClient()
  const [connectionState, setConnectionState] =
    useState<ConnectionState>('connecting')
  const [syncError, setSyncError] = useState<GameRoomError | null>(null)
  const [isRefreshing, setIsRefreshing] = useState(false)

  const wsRef = useRef<WebSocket | null>(null)
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const reconnectDelayRef = useRef(INITIAL_RECONNECT_DELAY_MS)
  const shouldReconnectRef = useRef(true)
  const etagRef = useRef<string | undefined>(initialData.etag)
  const lockVersionRef = useRef<number | undefined>(initialData.lockVersion)

  const buildEtag = useCallback(
    (lockVersion?: number) =>
      typeof lockVersion === 'number'
        ? `"game-${gameId}-v${lockVersion}"`
        : undefined,
    [gameId]
  )

  const applySnapshot = useCallback(
    (payload: GameRoomSnapshotPayload) => {
      etagRef.current = payload.etag ?? buildEtag(payload.lockVersion)
      lockVersionRef.current = payload.lockVersion ?? lockVersionRef.current
      setSyncError(null)
      // Update query cache - this is the single source of truth
      // Components using useGameRoomSnapshot will automatically re-render
      queryClient.setQueryData(queryKeys.games.snapshot(gameId), payload)
    },
    [buildEtag, gameId, queryClient]
  )

  const refreshSnapshot = useCallback(async () => {
    setIsRefreshing(true)
    try {
      const result = await getGameRoomSnapshotAction({
        gameId,
        etag: etagRef.current,
      })

      if (result.kind === 'ok') {
        applySnapshot(result.data)
      } else if (result.kind === 'not_modified') {
        // For not_modified, update timestamp in cache if data exists
        const cachedData = queryClient.getQueryData<GameRoomSnapshotPayload>(
          queryKeys.games.snapshot(gameId)
        )
        if (cachedData) {
          queryClient.setQueryData(queryKeys.games.snapshot(gameId), {
            ...cachedData,
            timestamp: new Date().toISOString(),
          })
        }
      } else {
        setSyncError({ message: result.message, traceId: result.traceId })
      }
    } catch (error) {
      logError('Manual snapshot refresh failed', error, { gameId })
      setSyncError({
        message:
          error instanceof Error
            ? error.message
            : 'Unable to refresh game state',
      })
    } finally {
      setIsRefreshing(false)
    }
  }, [applySnapshot, gameId, queryClient])

  const transformSnapshotMessage = useCallback(
    (message: SnapshotMessage): GameRoomSnapshotPayload => {
      const { data, viewer_seat } = message
      const lockVersion = data.lock_version
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
        lockVersion,
        etag: buildEtag(lockVersion),
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
          const errorMsg = parsed as ErrorMessage
          setSyncError({
            message: errorMsg.message ?? 'Realtime connection error',
            traceId: errorMsg.code,
          })
        }
      } catch (error) {
        logError('Failed to parse websocket payload', error, { gameId })
      }
    },
    [handleSnapshotMessage, gameId]
  )

  const scheduleReconnect = useCallback((connectFn: () => Promise<void>) => {
    if (!shouldReconnectRef.current) {
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

  const fetchWsToken = useCallback(async () => {
    const response = await fetch('/api/ws-token', {
      method: 'GET',
      cache: 'no-store',
    })
    if (!response.ok) {
      throw new Error('Unable to fetch realtime token')
    }
    const body = (await response.json()) as { token?: string }
    if (!body.token) {
      throw new Error('Realtime token missing from response')
    }
    return body.token
  }, [])

  const resolveWsUrl = useCallback(() => {
    // Validate WebSocket config before resolving URL
    // This ensures we fail early with a clear error if configuration is missing
    try {
      validateWebSocketConfig()
    } catch (error) {
      logError('WebSocket configuration validation failed', error, { gameId })
      // In development, throw to make the issue obvious
      if (process.env.NODE_ENV === 'development') {
        throw error
      }
      // In production, log but allow connection attempt (will fail with clear error)
    }
    return resolveWebSocketUrl()
  }, [gameId])

  const connect = useCallback(async () => {
    const wsBase = resolveWsUrl()
    shouldReconnectRef.current = true
    setConnectionState((state) =>
      state === 'disconnected' ? 'reconnecting' : 'connecting'
    )

    try {
      const token = await fetchWsToken()
      const url = `${wsBase}/ws/games/${gameId}?token=${encodeURIComponent(token)}`
      const ws = new WebSocket(url)
      wsRef.current = ws

      ws.onopen = () => {
        reconnectDelayRef.current = INITIAL_RECONNECT_DELAY_MS
        setConnectionState('connected')
      }

      ws.onmessage = handleMessageEvent

      ws.onerror = () => {
        setSyncError({
          message: 'Websocket connection error',
          traceId: undefined,
        })
      }

      ws.onclose = () => {
        wsRef.current = null
        if (!shouldReconnectRef.current) {
          setConnectionState('disconnected')
          return
        }
        scheduleReconnect(connect)
      }
    } catch (error) {
      logError('Failed to establish realtime connection', error, { gameId })
      setSyncError({
        message:
          error instanceof Error
            ? error.message
            : 'Failed to establish realtime connection',
      })
      scheduleReconnect(connect)
    }
  }, [
    fetchWsToken,
    gameId,
    handleMessageEvent,
    resolveWsUrl,
    scheduleReconnect,
  ])

  useEffect(() => {
    void connect()
    return () => {
      shouldReconnectRef.current = false
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current)
        reconnectTimeoutRef.current = null
      }
      wsRef.current?.close()
    }
  }, [connect])

  return {
    refreshSnapshot,
    connectionState,
    syncError,
    isRefreshing,
  }
}
