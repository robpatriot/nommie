'use client'

import { useEffect, useMemo, useRef } from 'react'
import { useTranslations } from 'next-intl'

import type { GameRoomState } from '@/lib/game-room/state'
import {
  selectBidConstraints,
  selectHostSeat,
  selectPlayerNames,
  selectSnapshot,
  selectViewerHand,
  selectViewerSeat,
} from '@/lib/game-room/state'
import Toast from '@/components/Toast'
import { useToast } from '@/hooks/useToast'
import { useGameSync } from '@/hooks/useGameSync'
import { useGameRoomState } from '@/hooks/queries/useGameRoomState'
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
  initialState: GameRoomState
  gameId: number
  requireCardConfirmation?: boolean
  trickDisplayDurationSeconds?: number | null
}

export function GameRoomClient({
  initialState,
  gameId,
  requireCardConfirmation = true,
  trickDisplayDurationSeconds = null,
}: GameRoomClientProps) {
  const {
    data: state = initialState,
    error: queryError,
    isLoading: isStateLoading,
  } = useGameRoomState(gameId, {
    initialData: initialState,
    etag: initialState.etag,
  })

  const {
    refreshStateFromHttp,
    syncError,
    isRefreshing: syncIsRefreshing,
    disconnect,
    connect,
    connectionState,
    reconnectAttempts,
    maxReconnectAttempts,
  } = useGameSync({ initialState, gameId })

  const { toasts, showToast, hideToast } = useToast()
  const tGame = useTranslations('game.gameRoom')

  // Combine errors from query and WebSocket (mutations handle their own errors)
  const combinedError = syncError ?? getGameRoomError(queryError)

  // Combine loading/refreshing states
  // Only show loading for initial loads (isLoading) or manual refreshes (syncIsRefreshing)
  // Background refetches won't trigger loading indicators
  const isRefreshing = syncIsRefreshing || isStateLoading

  const viewerSeatForInteractions = useMemo<Seat | null>(
    () => normalizeViewerSeat(selectViewerSeat(state)),
    [state]
  )

  const phase = selectSnapshot(state).phase
  const phaseName = phase.phase

  const hostSeat = selectHostSeat(state)
  const viewerIsHost =
    hostSeat !== null && viewerSeatForInteractions === hostSeat
  const canViewAiManager = viewerIsHost && isInitPhase(phase)

  const { hasMarkedReady, setHasMarkedReady, canMarkReady } =
    useGameRoomReadyState(state, viewerSeatForInteractions, phaseName)

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
    viewerSeat: viewerSeatForInteractions,
  })

  const { biddingControls, trumpControls, playControls } = useGameRoomControls({
    phase,
    viewerSeatForInteractions,
    bidConstraints: selectBidConstraints(state),
    handleSubmitBid,
    handleSelectTrump,
    handlePlayCard,
    isBidPending,
    isTrumpPending,
    isPlayPending,
  })

  const { aiSeatState } = useAiSeatManagement({
    gameId,
    state,
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
        snapshot={selectSnapshot(state)}
        playerNames={selectPlayerNames(state)}
        viewerSeat={selectViewerSeat(state)}
        viewerHand={selectViewerHand(state)}
        onRefresh={() => void refreshStateFromHttp()}
        isRefreshing={isRefreshing}
        error={combinedError}
        readyState={useMemo(
          () => ({
            canReady: canMarkReady,
            isPending: isReadyPending,
            hasMarked: hasMarkedReady,
            onReady: () => {
              void markReady()
            },
          }),
          [canMarkReady, isReadyPending, hasMarkedReady, markReady]
        )}
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
