import { useCallback, useEffect, useRef, useState } from 'react'

import {
  getGameRoomSnapshotAction,
  type GameRoomSnapshotPayload,
} from '@/app/actions/game-room-actions'
import type { GameRoomError } from '@/app/game/[gameId]/_components/game-room-view.types'
import type { BidConstraints, GameSnapshot, Seat } from '@/lib/game-room/types'
import { extractPlayerNames } from '@/utils/player-names'

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
  snapshot: GameRoomSnapshotPayload
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
  const [snapshot, setSnapshot] = useState<GameRoomSnapshotPayload>(initialData)
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
      setSnapshot(payload)
      setSyncError(null)
    },
    [buildEtag]
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
        setSnapshot((prev) => ({
          ...prev,
          timestamp: new Date().toISOString(),
        }))
      } else {
        setSyncError({ message: result.message, traceId: result.traceId })
      }
    } catch (error) {
      console.error('Manual snapshot refresh failed', error)
      setSyncError({
        message:
          error instanceof Error
            ? error.message
            : 'Unable to refresh game state',
      })
    } finally {
      setIsRefreshing(false)
    }
  }, [applySnapshot, gameId])

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
        console.error('Failed to parse websocket payload', error)
      }
    },
    [handleSnapshotMessage]
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
    const explicitBase = process.env.NEXT_PUBLIC_BACKEND_WS_URL
    if (explicitBase) {
      return explicitBase.replace(/\/$/, '')
    }
    const httpBase = process.env.NEXT_PUBLIC_BACKEND_BASE_URL
    if (httpBase) {
      // Convert http:// to ws:// and https:// to wss://
      return httpBase
        .replace(/\/$/, '')
        .replace(/^https?/, (match) => (match === 'https' ? 'wss' : 'ws'))
    }
    throw new Error(
      'NEXT_PUBLIC_BACKEND_WS_URL or NEXT_PUBLIC_BACKEND_BASE_URL must be configured'
    )
  }, [])

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
      console.error('Failed to establish realtime connection', error)
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
    snapshot,
    refreshSnapshot,
    connectionState,
    syncError,
    isRefreshing,
  }
}
