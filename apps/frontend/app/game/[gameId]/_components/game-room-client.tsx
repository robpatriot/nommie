'use client'

import { useMemo } from 'react'

import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import Toast from '@/components/Toast'
import { useToast } from '@/hooks/useToast'
import { useGameSync } from '@/hooks/useGameSync'
import { useGameRoomSnapshot } from '@/hooks/queries/useGameRoomSnapshot'
import type { Seat } from '@/lib/game-room/types'

import { GameRoomView } from './game-room-view'
import { getGameRoomError } from '@/lib/queries/query-error-handler'
import { useGameRoomReadyState } from './hooks/useGameRoomReadyState'
import { useSlowSyncIndicator } from './hooks/useSlowSyncIndicator'
import { useGameRoomActions } from './hooks/useGameRoomActions'
import { useGameRoomControls } from './hooks/useGameRoomControls'
import { useAiSeatManagement } from './hooks/useAiSeatManagement'

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
    isLoading: isSnapshotLoading,
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

  // Combine errors from query and WebSocket (mutations handle their own errors)
  const combinedError = syncError ?? getGameRoomError(queryError)

  // Combine loading/refreshing states
  // Only show loading for initial loads (isLoading) or manual refreshes (syncIsRefreshing)
  // Background refetches won't trigger loading indicators
  const isRefreshing = syncIsRefreshing || isSnapshotLoading

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
    isReadyPending,
    isBidPending,
    isTrumpPending,
    isPlayPending,
  } = useGameRoomActions({
    gameId,
    snapshot,
    canMarkReady,
    hasMarkedReady,
    setHasMarkedReady,
    showToast,
  })

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
    showToast,
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
