'use client'

import { useEffect, useMemo, useRef } from 'react'
import { useTranslations } from 'next-intl'

import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import Toast from '@/components/Toast'
import { useToast } from '@/hooks/useToast'
import { useGameSync } from '@/hooks/useGameSync'
import { useGameSnapshot } from '@/hooks/queries/useGameSnapshot'
import type { Seat } from '@/lib/game-room/types'

import { GameRoomView } from './game-room-view'
import { getGameRoomError } from '@/lib/queries/query-error-handler'
import { useGameRoomReadyState } from './hooks/useGameRoomReadyState'
import { useSlowSyncIndicator } from './hooks/useSlowSyncIndicator'
import { useGameRoomActions } from './hooks/useGameRoomActions'
import { useGameRoomControls } from './hooks/useGameRoomControls'
import { useAiSeatManagement } from './hooks/useAiSeatManagement'
import { normalizeViewerSeat } from './game-room/utils'
import { isInitPhase } from './game-room/phase-helpers'

interface GameRoomClientProps {
  initialData: GameRoomSnapshotPayload
  gameId: number
  requireCardConfirmation?: boolean
  trickDisplayDurationSeconds?: number | null
}

export function GameRoomClient({
  initialData,
  gameId,
  requireCardConfirmation = true,
  trickDisplayDurationSeconds = null,
}: GameRoomClientProps) {
  // Read snapshot from TanStack Query cache (single source of truth)
  // WebSocket updates will automatically update the cache and trigger re-renders
  const {
    data: snapshot = initialData,
    error: queryError,
    isLoading: isSnapshotLoading,
  } = useGameSnapshot(gameId, {
    initialData,
    etag: initialData.etag,
  })

  // Get WebSocket connection state and refresh function
  const {
    refreshSnapshot,
    syncError,
    isRefreshing: syncIsRefreshing,
    disconnect,
    connect,
    connectionState,
    reconnectAttempts,
    maxReconnectAttempts,
  } = useGameSync({ initialData, gameId })

  const { toasts, showToast, hideToast } = useToast()
  const tGame = useTranslations('game.gameRoom')

  // Combine errors from query and WebSocket (mutations handle their own errors)
  const combinedError = syncError ?? getGameRoomError(queryError)

  // Combine loading/refreshing states
  // Only show loading for initial loads (isLoading) or manual refreshes (syncIsRefreshing)
  // Background refetches won't trigger loading indicators
  const isRefreshing = syncIsRefreshing || isSnapshotLoading

  // Normalize viewer seat once and reuse
  const viewerSeatForInteractions = useMemo<Seat | null>(
    () => normalizeViewerSeat(snapshot.viewerSeat),
    [snapshot.viewerSeat]
  )

  const phase = snapshot.snapshot.phase
  const phaseName = phase.phase

  // AI registry query visibility
  const hostSeat: Seat = snapshot.hostSeat
  const viewerIsHost = viewerSeatForInteractions === hostSeat
  const canViewAiManager = viewerIsHost && isInitPhase(phase)

  // Ready state management
  const { hasMarkedReady, setHasMarkedReady, canMarkReady } =
    useGameRoomReadyState(snapshot, viewerSeatForInteractions, phaseName)

  // Slow sync indicator
  useSlowSyncIndicator({
    isRefreshing,
    showToast,
    hideToast,
  })

  // Reconnection status indicator
  const reconnectingToastIdRef = useRef<string | null>(null)
  useEffect(() => {
    if (connectionState === 'reconnecting' && reconnectAttempts > 0) {
      // Update or show reconnection toast
      if (reconnectingToastIdRef.current) {
        // Hide old toast and show new one with updated count
        hideToast(reconnectingToastIdRef.current)
      }
      const toastId = showToast(
        `Reconnecting... (${reconnectAttempts}/${maxReconnectAttempts})`,
        'warning'
      )
      reconnectingToastIdRef.current = toastId
    } else if (connectionState === 'connected') {
      // Clear reconnecting toast when successfully connected
      if (reconnectingToastIdRef.current) {
        hideToast(reconnectingToastIdRef.current)
        reconnectingToastIdRef.current = null
      }
    }
  }, [
    connectionState,
    reconnectAttempts,
    maxReconnectAttempts,
    showToast,
    hideToast,
  ])

  // Game room actions
  const {
    markReady,
    handleLeaveGame,
    handleSubmitBid,
    handleSelectTrump,
    handlePlayCard,
    isReadyPending,
    isLeavePending,
    isBidPending,
    isTrumpPending,
    isPlayPending,
  } = useGameRoomActions({
    gameId,
    canMarkReady,
    hasMarkedReady,
    setHasMarkedReady,
    showToast,
    disconnect,
    connect,
    phase,
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

  const handleCopyInvite = () => {
    const url = window.location.href
    void navigator.clipboard
      .writeText(url)
      .then(() => {
        showToast(tGame('setup.quickActions.copySuccess'), 'success')
      })
      .catch(() => {
        showToast(tGame('setup.quickActions.copyError'), 'error')
      })
  }

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
        trickDisplayDurationSeconds={trickDisplayDurationSeconds}
        onCopyInvite={handleCopyInvite}
        onLeaveGame={handleLeaveGame}
        isLeavePending={isLeavePending}
      />
      <Toast toasts={toasts} onClose={hideToast} />
    </>
  )
}
