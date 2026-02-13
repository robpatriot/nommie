import { useCallback, useEffect, useRef, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'

import { logError } from '@/lib/logging/error-logger'

import { getGameRoomStateAction } from '@/app/actions/game-room-actions'
import type { GameRoomError } from '@/app/game/[gameId]/_components/game-room-view.types'
import type {
  GameStateMsg,
  SubscribeMsg,
  UnsubscribeMsg,
  WireMsg,
} from '@/lib/game-room/protocol/types'
import { isGameStateMsg } from '@/lib/game-room/protocol/types'
import { queryKeys } from '@/lib/queries/query-keys'
import {
  type GameRoomState,
  gameStateMsgToRoomState,
  selectSnapshot,
  selectViewerSeat,
} from '@/lib/game-room/state'
import { useWebSocket } from '@/lib/providers/web-socket-provider'
import {
  getLwPendingAction,
  onYourTurn,
  setLwPendingAction,
} from '@/lib/queries/lw-cache'

export interface UseGameSyncResult {
  refreshStateFromHttp: () => Promise<void>
  connectionState: 'connecting' | 'connected' | 'reconnecting' | 'disconnected'
  syncError: GameRoomError | null
  reconnectAttempts: number
  maxReconnectAttempts: number
  isRefreshing: boolean
  disconnect: () => void
  connect: () => Promise<void>
}

interface UseGameSyncOptions {
  initialState: GameRoomState
  gameId: number
}

export function useGameSync({
  initialState,
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
  const etagRef = useRef<string | undefined>(initialState.etag)
  const lastWsVersionSeenRef = useRef<number | undefined>(undefined)
  const prevIsUsersTurnRef = useRef<boolean>(false)
  const didInitRef = useRef<number | null>(null)

  // Keep gameIdRef in sync
  useEffect(() => {
    gameIdRef.current = gameId
  }, [gameId])

  const applyState = useCallback(
    (roomState: GameRoomState) => {
      const currentGameId = gameIdRef.current
      const current = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(currentGameId)
      )
      if (
        current &&
        current.version !== undefined &&
        roomState.version !== undefined &&
        current.version >= roomState.version
      ) {
        return
      }

      if (roomState.etag !== undefined) etagRef.current = roomState.etag
      setLocalSyncError(null)
      queryClient.setQueryData(queryKeys.games.state(currentGameId), roomState)
    },
    [queryClient]
  )

  const computeMyTurnFromState = useCallback(
    (state: GameRoomState): boolean => {
      const viewerSeat = selectViewerSeat(state)
      if (viewerSeat === null) return false

      const phase = selectSnapshot(state).phase
      if (phase.phase === 'Bidding') {
        return phase.data.to_act === viewerSeat
      }
      if (phase.phase === 'TrumpSelect') {
        return phase.data.to_act === viewerSeat
      }
      if (phase.phase === 'Trick') {
        return phase.data.to_act === viewerSeat
      }
      return false
    },
    []
  )

  // Initialize edge detector from initial HTTP state (prevents false edges on navigation/reload).
  // Run only when gameId changes; initialState is seed-only, not live query state.
  useEffect(() => {
    if (didInitRef.current !== gameId) {
      didInitRef.current = gameId
      prevIsUsersTurnRef.current = computeMyTurnFromState(initialState)
      setLwPendingAction(queryClient, gameId, false)
    }
  }, [gameId, initialState, computeMyTurnFromState, queryClient])

  const refreshStateFromHttp = useCallback(async () => {
    setIsRefreshing(true)
    try {
      const result = await getGameRoomStateAction({
        gameId: gameIdRef.current,
        etag: etagRef.current,
      })

      if (result.kind === 'ok') {
        applyState(result.data)
      } else if (result.kind === 'not_modified') {
        // No cache write: 304 means nothing changed; avoid receivedAt-only churn.
      } else {
        setLocalSyncError({ message: result.message, traceId: result.traceId })
      }
    } catch (error) {
      logError('Manual state refresh failed', error, {
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
  }, [applyState])

  const handleGameStateMessage = useCallback(
    (message: GameStateMsg) => {
      if (
        message.topic?.kind !== 'game' ||
        message.topic.id !== gameIdRef.current
      ) {
        return
      }

      lastWsVersionSeenRef.current = message.version
      const roomState = gameStateMsgToRoomState(message, { source: 'ws' })
      applyState(roomState)

      const myTurn = computeMyTurnFromState(roomState)
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
    [applyState, computeMyTurnFromState, queryClient]
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
        void refreshStateFromHttp()
        return
      }
    })
  }, [
    registerHandler,
    handleGameStateMessage,
    queryClient,
    refreshStateFromHttp,
  ])

  useEffect(() => {
    etagRef.current = initialState.etag
  }, [initialState.etag])

  return {
    refreshStateFromHttp,
    connectionState,
    syncError,
    isRefreshing,
    disconnect: wsDisconnect,
    connect: wsConnect,
    reconnectAttempts,
    maxReconnectAttempts,
  }
}
