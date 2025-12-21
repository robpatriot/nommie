import { useCallback } from 'react'
import { useTranslations } from 'next-intl'
import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import type { Trump } from '@/lib/game-room/types'
import type { ToastMessage } from '@/components/Toast'
import {
  useMarkPlayerReady,
  useSubmitBid,
  useSelectTrump,
  useSubmitPlay,
} from '@/hooks/mutations/useGameRoomMutations'
import { toQueryError } from '@/lib/queries/query-error-handler'
import type { BackendApiError } from '@/lib/errors'

interface UseGameRoomActionsProps {
  gameId: number
  snapshot: GameRoomSnapshotPayload
  canMarkReady: boolean
  hasMarkedReady: boolean
  setHasMarkedReady: (value: boolean) => void
  showToast: (
    message: string,
    type: ToastMessage['type'],
    error?: BackendApiError
  ) => string
}

/**
 * Manages all game room action handlers (ready, bid, trump, play).
 * Uses TanStack Query mutation state for pending and error handling.
 */
export function useGameRoomActions({
  gameId,
  snapshot,
  canMarkReady,
  hasMarkedReady,
  setHasMarkedReady,
  showToast,
}: UseGameRoomActionsProps) {
  const t = useTranslations('toasts')
  const tErrors = useTranslations('toasts.gameRoom.errors')

  // Mutations
  const markPlayerReadyMutation = useMarkPlayerReady()
  const submitBidMutation = useSubmitBid()
  const selectTrumpMutation = useSelectTrump()
  const submitPlayMutation = useSubmitPlay()

  // Pending states from mutations
  const isReadyPending = markPlayerReadyMutation.isPending
  const isBidPending = submitBidMutation.isPending
  const isTrumpPending = selectTrumpMutation.isPending
  const isPlayPending = submitPlayMutation.isPending

  const markReady = useCallback(async () => {
    if (!canMarkReady || isReadyPending || hasMarkedReady) {
      return
    }

    try {
      await markPlayerReadyMutation.mutateAsync(gameId)
      setHasMarkedReady(true)
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

      if (snapshot.lockVersion === undefined) {
        showToast(tErrors('lockVersionRequiredBid'), 'error')
        return
      }

      try {
        await submitBidMutation.mutateAsync({
          gameId,
          bid,
          lockVersion: snapshot.lockVersion!,
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
      snapshot.lockVersion,
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

      if (snapshot.lockVersion === undefined) {
        showToast(tErrors('lockVersionRequiredTrump'), 'error')
        return
      }

      try {
        await selectTrumpMutation.mutateAsync({
          gameId,
          trump,
          lockVersion: snapshot.lockVersion!,
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
      snapshot.lockVersion,
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

      if (snapshot.lockVersion === undefined) {
        showToast(tErrors('lockVersionRequiredCard'), 'error')
        return
      }

      try {
        await submitPlayMutation.mutateAsync({
          gameId,
          card,
          lockVersion: snapshot.lockVersion!,
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
      snapshot.lockVersion,
      submitPlayMutation,
      showToast,
      t,
      tErrors,
    ]
  )

  return {
    // Action handlers
    markReady,
    handleSubmitBid,
    handleSelectTrump,
    handlePlayCard,
    // Pending states from mutations
    isReadyPending,
    isBidPending,
    isTrumpPending,
    isPlayPending,
  }
}
