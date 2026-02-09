import { useCallback, useEffect, useRef, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'

import { logError } from '@/lib/logging/error-logger'

import {
  getGameRoomSnapshotAction,
  type GameRoomSnapshotPayload,
} from '@/app/actions/game-room-actions'
import type { GameRoomError } from '@/app/game/[gameId]/_components/game-room-view.types'
import type {
  GameStateMsg,
  SubscribeMsg,
  UnsubscribeMsg,
  WireMsg,
} from '@/lib/game-room/protocol/types'
import { isGameStateMsg } from '@/lib/game-room/protocol/types'
import { queryKeys } from '@/lib/queries/query-keys'
import { gameStateMsgToSnapshotPayload } from '@/lib/game-room/protocol/transform'
import { useWebSocket } from '@/lib/providers/web-socket-provider'
import {
  getLwPendingAction,
  onYourTurn,
  setLwPendingAction,
} from '@/lib/queries/lw-cache'

export interface UseGameSyncResult {
  refreshSnapshot: () => Promise<void>
  connectionState: 'connecting' | 'connected' | 'reconnecting' | 'disconnected'
  syncError: GameRoomError | null
  reconnectAttempts: number
  maxReconnectAttempts: number
  isRefreshing: boolean
  disconnect: () => void
  connect: () => Promise<void>
}

interface UseGameSyncOptions {
  initialData: GameRoomSnapshotPayload
  gameId: number
}

export function useGameSync({
  initialData,
  gameId,
}: UseGameSyncOptions): UseGameSyncResult {
  const queryClient = useQueryClient()
  const {
    connectionState,
    syncError: wsSyncError,
    reconnectAttempts,
    maxReconnectAttempts,
    sendMessage,
    registerHandler,
    connect: wsConnect,
    disconnect: wsDisconnect,
  } = useWebSocket()

  const [localSyncError, setLocalSyncError] = useState<GameRoomError | null>(
    null
  )
  const [isRefreshing, setIsRefreshing] = useState(false)

  // Combined error: Prefer local subscription/message errors, fallback to global WS errors
  const syncError = localSyncError ?? wsSyncError

  const gameIdRef = useRef(gameId)
  const etagRef = useRef<string | undefined>(initialData.etag)
  const lastWsVersionSeenRef = useRef<number | undefined>(undefined)
  const prevIsUsersTurnRef = useRef<boolean>(false)

  // Keep gameIdRef in sync
  useEffect(() => {
    gameIdRef.current = gameId
  }, [gameId])

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
      setLocalSyncError(null)
      queryClient.setQueryData(queryKeys.games.snapshot(currentGameId), payload)
    },
    [buildEtag, queryClient]
  )

  const computeMyTurn = useCallback(
    (payload: GameRoomSnapshotPayload): boolean => {
      if (payload.viewerSeat === null) return false

      const phase = payload.snapshot.phase
      if (phase.phase === 'Bidding') {
        return phase.data.to_act === payload.viewerSeat
      }
      if (phase.phase === 'TrumpSelect') {
        return phase.data.to_act === payload.viewerSeat
      }
      if (phase.phase === 'Trick') {
        return phase.data.to_act === payload.viewerSeat
      }
      return false
    },
    []
  )

  // Initialize edge detector from initial HTTP snapshot (prevents false edges on navigation/reload).
  useEffect(() => {
    prevIsUsersTurnRef.current = computeMyTurn(initialData)
    setLwPendingAction(queryClient, gameId, false)
  }, [computeMyTurn, gameId, initialData, queryClient])

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
        setLocalSyncError({ message: result.message, traceId: result.traceId })
      }
    } catch (error) {
      logError('Manual snapshot refresh failed', error, {
        gameId: gameIdRef.current,
      })
      setLocalSyncError({
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
      if (
        message.topic?.kind !== 'game' ||
        message.topic.id !== gameIdRef.current
      ) {
        return
      }

      lastWsVersionSeenRef.current = message.version
      const payload = gameStateMsgToSnapshotPayload(message)
      applySnapshot(payload)

      // LW cache rules: eligibility derived from game_state for the current game.
      const myTurn = computeMyTurn(payload)
      const prev = prevIsUsersTurnRef.current
      const pendingAction = getLwPendingAction(queryClient, gameIdRef.current)
      prevIsUsersTurnRef.current = myTurn

      if (pendingAction) {
        setLwPendingAction(queryClient, gameIdRef.current, false)
      }

      if (myTurn && (pendingAction || !prev)) {
        // Edge-trigger: treat as `your_turn` for the current game.
        void onYourTurn(queryClient, { gameId: gameIdRef.current })
      }
    },
    [applySnapshot, computeMyTurn, queryClient]
  )

  // Handle subscription lifecycle
  useEffect(() => {
    if (connectionState === 'connected') {
      const subscribeMsg: SubscribeMsg = {
        type: 'subscribe',
        topic: { kind: 'game', id: gameId },
      }
      sendMessage(subscribeMsg)

      return () => {
        const unsubscribeMsg: UnsubscribeMsg = {
          type: 'unsubscribe',
          topic: { kind: 'game', id: gameId },
        }
        sendMessage(unsubscribeMsg)
      }
    }
  }, [gameId, connectionState, sendMessage])

  // Register for messages
  useEffect(() => {
    return registerHandler((msg: WireMsg) => {
      if (isGameStateMsg(msg)) {
        handleGameStateMessage(msg)
        return
      }

      // handle other message types
      if (msg.type === 'error') {
        // If the server rejected an action, avoid leaving a pendingAction latch set.
        setLwPendingAction(queryClient, gameIdRef.current, false)
        // Fallback to HTTP refresh on server-side errors
        void refreshSnapshot()
        return
      }
    })
  }, [registerHandler, handleGameStateMessage, queryClient, refreshSnapshot])

  useEffect(() => {
    etagRef.current = initialData.etag
  }, [initialData.etag])

  return {
    refreshSnapshot,
    connectionState,
    syncError,
    isRefreshing,
    disconnect: wsDisconnect,
    connect: wsConnect,
    reconnectAttempts,
    maxReconnectAttempts,
  }
}
