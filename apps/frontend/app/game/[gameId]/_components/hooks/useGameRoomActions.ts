import { useCallback } from 'react'
import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import type { Trump } from '@/lib/game-room/types'
import { useToast } from '@/hooks/useToast'
import {
  useMarkPlayerReady,
  useSubmitBid,
  useSelectTrump,
  useSubmitPlay,
} from '@/hooks/mutations/useGameRoomMutations'
import { toQueryError } from '@/lib/queries/query-error-handler'

interface UseGameRoomActionsProps {
  gameId: number
  snapshot: GameRoomSnapshotPayload
  canMarkReady: boolean
  hasMarkedReady: boolean
  setHasMarkedReady: (value: boolean) => void
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
}: UseGameRoomActionsProps) {
  const { showToast } = useToast()

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
      const backendError = toQueryError(err, 'Unable to mark ready')
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
  ])

  const handleSubmitBid = useCallback(
    async (bid: number) => {
      if (isBidPending) {
        return
      }

      if (snapshot.lockVersion === undefined) {
        showToast('Lock version is required to submit bid', 'error')
        return
      }

      try {
        await submitBidMutation.mutateAsync({
          gameId,
          bid,
          lockVersion: snapshot.lockVersion!,
        })
        showToast('Bid submitted', 'success')
      } catch (err) {
        const backendError = toQueryError(err, 'Failed to submit bid')
        showToast(backendError.message, 'error', backendError)
      }
    },
    [gameId, isBidPending, snapshot.lockVersion, submitBidMutation, showToast]
  )

  const handleSelectTrump = useCallback(
    async (trump: Trump) => {
      if (isTrumpPending) {
        return
      }

      if (snapshot.lockVersion === undefined) {
        showToast('Lock version is required to select trump', 'error')
        return
      }

      try {
        await selectTrumpMutation.mutateAsync({
          gameId,
          trump,
          lockVersion: snapshot.lockVersion!,
        })
        showToast('Trump selected', 'success')
      } catch (err) {
        const backendError = toQueryError(err, 'Failed to select trump')
        showToast(backendError.message, 'error', backendError)
      }
    },
    [
      gameId,
      isTrumpPending,
      snapshot.lockVersion,
      selectTrumpMutation,
      showToast,
    ]
  )

  const handlePlayCard = useCallback(
    async (card: string) => {
      if (isPlayPending) {
        return
      }

      if (snapshot.lockVersion === undefined) {
        showToast('Lock version is required to play card', 'error')
        return
      }

      try {
        await submitPlayMutation.mutateAsync({
          gameId,
          card,
          lockVersion: snapshot.lockVersion!,
        })
        showToast('Card played', 'success')
      } catch (err) {
        const backendError = toQueryError(err, 'Failed to play card')
        showToast(backendError.message, 'error', backendError)
      }
    },
    [gameId, isPlayPending, snapshot.lockVersion, submitPlayMutation, showToast]
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
