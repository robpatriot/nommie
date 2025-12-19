import { useCallback, useState } from 'react'
import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import type { GameRoomError } from '../game-room-view.types'
import type { Trump } from '@/lib/game-room/types'
import { useToast } from '@/hooks/useToast'
import {
  useMarkPlayerReady,
  useSubmitBid,
  useSelectTrump,
  useSubmitPlay,
} from '@/hooks/mutations/useGameRoomMutations'
import { toQueryError } from '@/lib/queries/query-error-handler'

type PendingAction = 'ready' | 'bid' | 'trump' | 'play' | 'ai' | null

interface UseGameRoomActionsProps {
  gameId: number
  snapshot: GameRoomSnapshotPayload
  canMarkReady: boolean
  hasMarkedReady: boolean
  setHasMarkedReady: (value: boolean) => void
  runExclusiveAction: (
    actionType: Exclude<PendingAction, null>,
    actionFn: () => Promise<void>
  ) => Promise<void>
}

/**
 * Manages all game room action handlers (ready, bid, trump, play).
 * Handles pending state, error state, and exclusive action execution.
 */
export function useGameRoomActions({
  gameId,
  snapshot,
  canMarkReady,
  hasMarkedReady,
  setHasMarkedReady,
  runExclusiveAction,
}: UseGameRoomActionsProps) {
  const [actionError, setActionError] = useState<GameRoomError | null>(null)
  const { showToast } = useToast()

  // Mutations
  const markPlayerReadyMutation = useMarkPlayerReady()
  const submitBidMutation = useSubmitBid()
  const selectTrumpMutation = useSelectTrump()
  const submitPlayMutation = useSubmitPlay()

  // Pending states
  const isReadyPending = markPlayerReadyMutation.isPending
  const isBidPending = submitBidMutation.isPending
  const isTrumpPending = selectTrumpMutation.isPending
  const isPlayPending = submitPlayMutation.isPending

  const markReady = useCallback(async () => {
    if (!canMarkReady || isReadyPending || hasMarkedReady) {
      return
    }

    await runExclusiveAction('ready', async () => {
      setActionError(null)
      try {
        await markPlayerReadyMutation.mutateAsync(gameId)
        setHasMarkedReady(true)
      } catch (err) {
        const backendError = toQueryError(err, 'Unable to mark ready')
        setActionError({
          message: backendError.message,
          traceId: backendError.traceId,
        })
        showToast(backendError.message, 'error', backendError)
      }
    })
  }, [
    canMarkReady,
    gameId,
    hasMarkedReady,
    isReadyPending,
    runExclusiveAction,
    markPlayerReadyMutation,
    setHasMarkedReady,
    showToast,
  ])

  const handleSubmitBid = useCallback(
    async (bid: number) => {
      if (isBidPending) {
        return
      }

      await runExclusiveAction('bid', async () => {
        setActionError(null)
        if (snapshot.lockVersion === undefined) {
          setActionError({
            message: 'Lock version is required to submit bid',
          })
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
      })
    },
    [
      gameId,
      isBidPending,
      runExclusiveAction,
      snapshot.lockVersion,
      submitBidMutation,
      showToast,
    ]
  )

  const handleSelectTrump = useCallback(
    async (trump: Trump) => {
      if (isTrumpPending) {
        return
      }

      await runExclusiveAction('trump', async () => {
        setActionError(null)
        if (snapshot.lockVersion === undefined) {
          setActionError({
            message: 'Lock version is required to select trump',
          })
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
      })
    },
    [
      gameId,
      isTrumpPending,
      runExclusiveAction,
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

      await runExclusiveAction('play', async () => {
        setActionError(null)
        if (snapshot.lockVersion === undefined) {
          setActionError({
            message: 'Lock version is required to play card',
          })
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
      })
    },
    [
      gameId,
      isPlayPending,
      runExclusiveAction,
      snapshot.lockVersion,
      submitPlayMutation,
      showToast,
    ]
  )

  return {
    // Action handlers
    markReady,
    handleSubmitBid,
    handleSelectTrump,
    handlePlayCard,
    // Pending states
    isReadyPending,
    isBidPending,
    isTrumpPending,
    isPlayPending,
    // Error state
    actionError,
  }
}
