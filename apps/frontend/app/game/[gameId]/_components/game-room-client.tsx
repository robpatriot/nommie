'use client'

import { useCallback, useEffect, useMemo, useState } from 'react'

import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import Toast from '@/components/Toast'
import { useToast } from '@/hooks/useToast'
import { useGameSync } from '@/hooks/useGameSync'
import { useGameRoomSnapshot } from '@/hooks/queries/useGameRoomSnapshot'
import type { Seat } from '@/lib/game-room/types'

import { GameRoomView } from './game-room-view'
import type { GameRoomError } from './game-room-view.types'
import {
  useAddAiSeat,
  useUpdateAiSeat,
  useRemoveAiSeat,
} from '@/hooks/mutations/useGameRoomMutations'
import { getGameRoomError } from '@/lib/queries/query-error-handler'
import { useGameRoomReadyState } from './hooks/useGameRoomReadyState'
import { useSlowSyncIndicator } from './hooks/useSlowSyncIndicator'
import { useGameRoomActions } from './hooks/useGameRoomActions'
import { useGameRoomControls } from './hooks/useGameRoomControls'
import { useAiSeatManagement } from './hooks/useAiSeatManagement'

type PendingAction = 'ready' | 'bid' | 'trump' | 'play' | 'ai' | null

interface GameRoomClientProps {
  initialData: GameRoomSnapshotPayload
  gameId: number
  requireCardConfirmation?: boolean
}

export function GameRoomClient({
  initialData,
  gameId,
  requireCardConfirmation = true,
}: GameRoomClientProps) {
  // Read snapshot from TanStack Query cache (single source of truth)
  // WebSocket updates will automatically update the cache and trigger re-renders
  const {
    data: snapshot = initialData,
    error: queryError,
    isFetching: isSnapshotFetching,
  } = useGameRoomSnapshot(gameId, {
    initialData,
    etag: initialData.etag,
  })

  // Get WebSocket connection state and refresh function
  const {
    refreshSnapshot,
    syncError,
    isRefreshing: syncIsRefreshing,
  } = useGameSync({ initialData, gameId })

  const { toasts, showToast, hideToast } = useToast()
  const [pendingAction, setPendingAction] = useState<PendingAction>(null)
  const [actionError, setActionError] = useState<GameRoomError | null>(null)

  // Combine errors from query, WebSocket, and actions
  const combinedError = actionError ?? syncError ?? getGameRoomError(queryError)

  // Combine loading/refreshing states
  const isRefreshing = syncIsRefreshing || isSnapshotFetching

  // Calculate viewer seat once and reuse
  const viewerSeatForInteractions = useMemo<Seat | null>(
    () =>
      typeof snapshot.viewerSeat === 'number'
        ? (snapshot.viewerSeat as Seat)
        : null,
    [snapshot.viewerSeat]
  )

  const phase = snapshot.snapshot.phase
  const phaseName = phase.phase

  // AI registry query visibility
  const hostSeat: Seat = snapshot.hostSeat
  const viewerIsHost = viewerSeatForInteractions === hostSeat
  const canViewAiManager = viewerIsHost && phaseName === 'Init'

  // AI mutations for pending state calculation
  const addAiSeatMutation = useAddAiSeat()
  const updateAiSeatMutation = useUpdateAiSeat()
  const removeAiSeatMutation = useRemoveAiSeat()

  // Calculate combined pending states (pendingAction + mutation states)
  const isAiPending =
    pendingAction === 'ai' ||
    addAiSeatMutation.isPending ||
    updateAiSeatMutation.isPending ||
    removeAiSeatMutation.isPending

  // Exclusive action runner (shared across all actions)
  const finishAction = useCallback(() => {
    setPendingAction(null)
  }, [])

  const runExclusiveAction = useCallback(
    async (
      actionType: Exclude<PendingAction, null>,
      actionFn: () => Promise<void>
    ) => {
      if (pendingAction) {
        return
      }
      setPendingAction(actionType)
      setActionError(null)
      try {
        await actionFn()
      } finally {
        finishAction()
      }
    },
    [finishAction, pendingAction]
  )

  // Ready state management
  const { hasMarkedReady, setHasMarkedReady, canMarkReady } =
    useGameRoomReadyState(snapshot, viewerSeatForInteractions, phaseName)

  // Slow sync indicator
  useSlowSyncIndicator({
    isRefreshing,
    showToast,
    hideToast,
  })

  // Game room actions
  const {
    markReady,
    handleSubmitBid,
    handleSelectTrump,
    handlePlayCard,
    isReadyPending: isReadyPendingFromHook,
    isBidPending: isBidPendingFromHook,
    isTrumpPending: isTrumpPendingFromHook,
    isPlayPending: isPlayPendingFromHook,
    actionError: actionErrorFromHook,
  } = useGameRoomActions({
    gameId,
    snapshot,
    canMarkReady,
    hasMarkedReady,
    setHasMarkedReady,
    runExclusiveAction,
  })

  // Sync actionError from hook
  useEffect(() => {
    setActionError(actionErrorFromHook)
  }, [actionErrorFromHook])

  // Calculate combined pending states
  const isReadyPending = pendingAction === 'ready' || isReadyPendingFromHook
  const isBidPending = pendingAction === 'bid' || isBidPendingFromHook
  const isTrumpPending = pendingAction === 'trump' || isTrumpPendingFromHook
  const isPlayPending = pendingAction === 'play' || isPlayPendingFromHook

  // Game room controls
  const { biddingControls, trumpControls, playControls } = useGameRoomControls({
    phase,
    viewerSeatForInteractions,
    snapshot,
    handleSubmitBid,
    handleSelectTrump,
    handlePlayCard,
    isBidPending,
    isTrumpPending,
    isPlayPending,
  })

  // AI seat management
  const { aiSeatState } = useAiSeatManagement({
    gameId,
    snapshot,
    canViewAiManager,
    isAiPending,
    runExclusiveAction,
  })

  return (
    <>
      <GameRoomView
        gameId={gameId}
        snapshot={snapshot.snapshot}
        playerNames={snapshot.playerNames}
        viewerSeat={snapshot.viewerSeat ?? null}
        viewerHand={snapshot.viewerHand}
        onRefresh={() => void refreshSnapshot()}
        isRefreshing={isRefreshing}
        error={combinedError}
        readyState={{
          canReady: canMarkReady,
          isPending: isReadyPending,
          hasMarked: hasMarkedReady,
          onReady: () => {
            void markReady()
          },
        }}
        biddingState={biddingControls}
        trumpState={trumpControls}
        playState={playControls}
        aiSeatState={aiSeatState}
        requireCardConfirmation={requireCardConfirmation}
      />
      <Toast toasts={toasts} onClose={hideToast} />
    </>
  )
}
