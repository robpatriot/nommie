import { useCallback } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useTranslations } from 'next-intl'
import { useRouter } from 'next/navigation'
import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import type { PhaseSnapshot, Trump } from '@/lib/game-room/types'
import type { ToastMessage } from '@/components/Toast'
import { isActiveGame } from '../game-room/phase-helpers'
import {
  useMarkPlayerReady,
  useLeaveGame,
  useSubmitBid,
  useSelectTrump,
  useSubmitPlay,
} from '@/hooks/mutations/useGameRoomMutations'
import { queryKeys } from '@/lib/queries/query-keys'
import { toQueryError } from '@/lib/queries/query-error-handler'
import type { BackendApiError } from '@/lib/errors'

interface UseGameRoomActionsProps {
  gameId: number
  canMarkReady: boolean
  hasMarkedReady: boolean
  setHasMarkedReady: (value: boolean) => void
  showToast: (
    message: string,
    type: ToastMessage['type'],
    error?: BackendApiError
  ) => string
  disconnect: () => void
  connect: () => Promise<void>
  phase: PhaseSnapshot
}

/**
 * Manages all game room action handlers (ready, bid, trump, play).
 * Uses TanStack Query mutation state for pending and error handling.
 */
export function useGameRoomActions({
  gameId,
  canMarkReady,
  hasMarkedReady,
  setHasMarkedReady,
  showToast,
  disconnect,
  connect,
  phase,
}: UseGameRoomActionsProps) {
  const queryClient = useQueryClient()
  const router = useRouter()
  const t = useTranslations('toasts')
  const tErrors = useTranslations('toasts.gameRoom.errors')

  // Mutations
  const markPlayerReadyMutation = useMarkPlayerReady()
  const leaveGameMutation = useLeaveGame()
  const submitBidMutation = useSubmitBid()
  const selectTrumpMutation = useSelectTrump()
  const submitPlayMutation = useSubmitPlay()

  // Pending states from mutations
  const isReadyPending = markPlayerReadyMutation.isPending
  const isLeavePending = leaveGameMutation.isPending
  const isBidPending = submitBidMutation.isPending
  const isTrumpPending = selectTrumpMutation.isPending
  const isPlayPending = submitPlayMutation.isPending

  const markReady = useCallback(async () => {
    if (!canMarkReady || isReadyPending) {
      return
    }

    const newReadyState = !hasMarkedReady

    try {
      await markPlayerReadyMutation.mutateAsync({
        gameId,
        isReady: newReadyState,
      })
      setHasMarkedReady(newReadyState)
    } catch (err) {
      const backendError = toQueryError(err, tErrors('unableToMarkReady'))
      showToast(backendError.message, 'error', backendError)
    }
  }, [
    canMarkReady,
    gameId,
    hasMarkedReady,
    isReadyPending,
    markPlayerReadyMutation,
    setHasMarkedReady,
    showToast,
    tErrors,
  ])

  const handleSubmitBid = useCallback(
    async (bid: number) => {
      if (isBidPending) {
        return
      }

      // Read version directly from cache at request time to avoid stale closures
      const cachedSnapshot = queryClient.getQueryData<GameRoomSnapshotPayload>(
        queryKeys.games.snapshot(gameId)
      )
      const currentVersion = cachedSnapshot?.version

      if (currentVersion === undefined) {
        showToast(tErrors('versionRequiredBid'), 'error')
        return
      }

      try {
        await submitBidMutation.mutateAsync({
          gameId,
          bid,
          version: currentVersion,
        })
        showToast(t('gameRoom.bidSubmitted'), 'success')
      } catch (err) {
        const backendError = toQueryError(err, tErrors('failedToSubmitBid'))
        showToast(backendError.message, 'error', backendError)
      }
    },
    [
      gameId,
      isBidPending,
      queryClient,
      submitBidMutation,
      showToast,
      t,
      tErrors,
    ]
  )

  const handleSelectTrump = useCallback(
    async (trump: Trump) => {
      if (isTrumpPending) {
        return
      }

      // Read version directly from cache at request time to avoid stale closures
      const cachedSnapshot = queryClient.getQueryData<GameRoomSnapshotPayload>(
        queryKeys.games.snapshot(gameId)
      )
      const currentVersion = cachedSnapshot?.version

      if (currentVersion === undefined) {
        showToast(tErrors('versionRequiredTrump'), 'error')
        return
      }

      try {
        await selectTrumpMutation.mutateAsync({
          gameId,
          trump,
          version: currentVersion,
        })
        showToast(t('gameRoom.trumpSelected'), 'success')
      } catch (err) {
        const backendError = toQueryError(err, tErrors('failedToSelectTrump'))
        showToast(backendError.message, 'error', backendError)
      }
    },
    [
      gameId,
      isTrumpPending,
      queryClient,
      selectTrumpMutation,
      showToast,
      t,
      tErrors,
    ]
  )

  const handlePlayCard = useCallback(
    async (card: string) => {
      if (isPlayPending) {
        return
      }

      // Read version directly from cache at request time to avoid stale closures
      const cachedSnapshot = queryClient.getQueryData<GameRoomSnapshotPayload>(
        queryKeys.games.snapshot(gameId)
      )
      const currentVersion = cachedSnapshot?.version

      if (currentVersion === undefined) {
        showToast(tErrors('versionRequiredCard'), 'error')
        return
      }

      try {
        await submitPlayMutation.mutateAsync({
          gameId,
          card,
          version: currentVersion,
        })
        showToast(t('gameRoom.cardPlayed'), 'success')
      } catch (err) {
        const backendError = toQueryError(err, tErrors('failedToPlayCard'))
        showToast(backendError.message, 'error', backendError)
      }
    },
    [
      gameId,
      isPlayPending,
      queryClient,
      submitPlayMutation,
      showToast,
      t,
      tErrors,
    ]
  )

  const handleLeaveGame = useCallback(async () => {
    if (isLeavePending) {
      return
    }

    // Check if game is active (not in Init/Lobby phase)
    const gameIsActive = isActiveGame(phase)

    // If game is active, show confirmation dialog
    if (gameIsActive) {
      const confirmed = window.confirm(
        tErrors('leaveActiveGameConfirmation') ||
          'Are you sure you want to leave? An AI will take over your seat and continue playing.'
      )
      if (!confirmed) {
        return
      }
    }

    // Close WebSocket BEFORE leaving to prevent broadcasts from reaching non-member
    disconnect()

    try {
      // Read version directly from cache at request time to avoid stale closures
      const cachedSnapshot = queryClient.getQueryData<GameRoomSnapshotPayload>(
        queryKeys.games.snapshot(gameId)
      )
      const currentVersion = cachedSnapshot?.version

      if (currentVersion === undefined) {
        showToast(tErrors('versionRequired'), 'error')
        connect() // Reconnect since we didn't leave
        return
      }

      await leaveGameMutation.mutateAsync({
        gameId,
        version: currentVersion,
      })
      showToast(t('gameRoom.leftGameSuccess'), 'success')
      router.push('/lobby')
    } catch (err) {
      // Reconnect since leave failed (they're still in the game)
      connect()

      // Show error message
      const backendError = toQueryError(err, tErrors('unableToLeaveGame'))
      showToast(backendError.message, 'error', backendError)
      // Don't navigate - keep them on the game page since they're still in it
    }
  }, [
    gameId,
    isLeavePending,
    leaveGameMutation,
    router,
    showToast,
    t,
    tErrors,
    disconnect,
    connect,
    phase,
    queryClient,
  ])

  return {
    // Action handlers
    markReady,
    handleLeaveGame,
    handleSubmitBid,
    handleSelectTrump,
    handlePlayCard,
    // Pending states from mutations
    isReadyPending,
    isLeavePending,
    isBidPending,
    isTrumpPending,
    isPlayPending,
  }
}
