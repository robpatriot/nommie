'use client'

import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useRouter } from 'next/navigation'
import {
  markPlayerReadyAction,
  leaveGameAction,
  rejoinGameAction,
  submitBidAction,
  selectTrumpAction,
  submitPlayAction,
  addAiSeatAction,
  updateAiSeatAction,
  removeAiSeatAction,
  type SubmitBidRequest,
  type SelectTrumpRequest,
  type SubmitPlayRequest,
  type ManageAiSeatRequest,
} from '@/app/actions/game-room-actions'
import { handleActionResultError } from '@/lib/queries/query-error-handler'
import { queryKeys } from '@/lib/queries/query-keys'
import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'

/**
 * Mutation hook to set player ready status.
 * Invalidates game snapshot cache on success.
 */
export function useMarkPlayerReady() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({
      gameId,
      isReady,
    }: {
      gameId: number
      isReady: boolean
    }): Promise<void> => {
      const result = await markPlayerReadyAction(gameId, isReady)
      if (result.kind === 'error') {
        throw handleActionResultError(result)
      }
    },
    onSuccess: () => {
      // Invalidate last active game so header button updates
      queryClient.invalidateQueries({
        queryKey: queryKeys.games.waitingLongest(),
      })
    },
  })
}

/**
 * Mutation hook to leave a game.
 * Invalidates game snapshot cache on success and navigates to lobby.
 */
export function useLeaveGame() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (request: {
      gameId: number
      version: number
    }): Promise<void> => {
      const result = await leaveGameAction(request.gameId, request.version)
      if (result.kind === 'error') {
        throw handleActionResultError(result)
      }
    },
    onSuccess: () => {
      // Only invalidate last active game for header button (fire-and-forget, doesn't affect game room)
      queryClient.invalidateQueries({
        queryKey: queryKeys.games.waitingLongest(),
      })
    },
  })
}

/**
 * Mutation hook to rejoin a game.
 * Invalidates game snapshot cache on success and navigates to game room.
 */
export function useRejoinGame() {
  const queryClient = useQueryClient()
  const router = useRouter()

  return useMutation({
    mutationFn: async (request: {
      gameId: number
      version: number
    }): Promise<void> => {
      const result = await rejoinGameAction({
        gameId: request.gameId,
        version: request.version,
      })
      if (result.kind === 'error') {
        throw handleActionResultError(result)
      }
    },
    onSuccess: (_, request) => {
      // Only invalidate snapshot for the game we're joining (fire-and-forget, doesn't affect lobby)
      // and last active game for header button.
      queryClient.invalidateQueries({
        queryKey: queryKeys.games.snapshot(request.gameId),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.games.waitingLongest(),
      })
      router.push(`/game/${request.gameId}`)
    },
  })
}

/**
 * Mutation hook to submit a bid.
 * Uses optimistic updates to immediately show the bid in the UI.
 * Invalidates game snapshot cache on success.
 */
export function useSubmitBid() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (request: SubmitBidRequest): Promise<void> => {
      const result = await submitBidAction(request)
      if (result.kind === 'error') {
        throw handleActionResultError(result)
      }
    },
    onMutate: async (request) => {
      // Cancel outgoing refetches to avoid overwriting optimistic update
      await queryClient.cancelQueries({
        queryKey: queryKeys.games.snapshot(request.gameId),
      })

      // Snapshot current value for rollback
      const previousSnapshot =
        queryClient.getQueryData<GameRoomSnapshotPayload>(
          queryKeys.games.snapshot(request.gameId)
        )

      if (!previousSnapshot) {
        return { previousSnapshot: undefined }
      }

      // Skip optimistic update if viewerSeat is null (user shouldn't be able to bid anyway)
      if (
        previousSnapshot.viewerSeat === null ||
        previousSnapshot.snapshot.phase.phase !== 'Bidding'
      ) {
        return { previousSnapshot }
      }

      // Optimistically update bid
      const updatedSnapshot: GameRoomSnapshotPayload = {
        ...previousSnapshot,
        snapshot: {
          ...previousSnapshot.snapshot,
          phase: {
            phase: 'Bidding',
            data: {
              ...previousSnapshot.snapshot.phase.data,
              bids: previousSnapshot.snapshot.phase.data.bids.map((bid, idx) =>
                idx === previousSnapshot.viewerSeat ? request.bid : bid
              ) as [number | null, number | null, number | null, number | null],
              round: {
                ...previousSnapshot.snapshot.phase.data.round,
                bids: previousSnapshot.snapshot.phase.data.round.bids.map(
                  (bid, idx) =>
                    idx === previousSnapshot.viewerSeat ? request.bid : bid
                ) as [
                  number | null,
                  number | null,
                  number | null,
                  number | null,
                ],
              },
            },
          },
        },
      }

      queryClient.setQueryData(
        queryKeys.games.snapshot(request.gameId),
        updatedSnapshot
      )

      return { previousSnapshot }
    },
    onError: (err, request, context) => {
      // Rollback on error
      if (context?.previousSnapshot) {
        queryClient.setQueryData(
          queryKeys.games.snapshot(request.gameId),
          context.previousSnapshot
        )
      }
    },
    onSuccess: () => {
      // Invalidate last active game so header button updates
      queryClient.invalidateQueries({
        queryKey: queryKeys.games.waitingLongest(),
      })
    },
  })
}

/**
 * Mutation hook to select trump suit.
 * Uses optimistic updates to immediately show the trump suit in the UI.
 * Invalidates game snapshot cache on success.
 */
export function useSelectTrump() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (request: SelectTrumpRequest): Promise<void> => {
      const result = await selectTrumpAction(request)
      if (result.kind === 'error') {
        throw handleActionResultError(result)
      }
    },
    onMutate: async (request) => {
      // Cancel outgoing refetches to avoid overwriting optimistic update
      await queryClient.cancelQueries({
        queryKey: queryKeys.games.snapshot(request.gameId),
      })

      // Snapshot current value for rollback
      const previousSnapshot =
        queryClient.getQueryData<GameRoomSnapshotPayload>(
          queryKeys.games.snapshot(request.gameId)
        )

      if (!previousSnapshot) {
        return { previousSnapshot: undefined }
      }

      // Skip optimistic update if not in TrumpSelect phase
      if (previousSnapshot.snapshot.phase.phase !== 'TrumpSelect') {
        return { previousSnapshot }
      }

      // Optimistically update trump suit
      const updatedSnapshot: GameRoomSnapshotPayload = {
        ...previousSnapshot,
        snapshot: {
          ...previousSnapshot.snapshot,
          phase: {
            phase: 'TrumpSelect',
            data: {
              ...previousSnapshot.snapshot.phase.data,
              round: {
                ...previousSnapshot.snapshot.phase.data.round,
                trump: request.trump,
              },
            },
          },
        },
      }

      queryClient.setQueryData(
        queryKeys.games.snapshot(request.gameId),
        updatedSnapshot
      )

      return { previousSnapshot }
    },
    onError: (err, request, context) => {
      // Rollback on error
      if (context?.previousSnapshot) {
        queryClient.setQueryData(
          queryKeys.games.snapshot(request.gameId),
          context.previousSnapshot
        )
      }
    },
    onSuccess: () => {
      // Invalidate last active game so header button updates
      queryClient.invalidateQueries({
        queryKey: queryKeys.games.waitingLongest(),
      })
    },
  })
}

/**
 * Mutation hook to submit a card play.
 * Uses optimistic updates to immediately show the card in the trick.
 * Invalidates game snapshot cache on success.
 */
export function useSubmitPlay() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (request: SubmitPlayRequest): Promise<void> => {
      const result = await submitPlayAction(request)
      if (result.kind === 'error') {
        throw handleActionResultError(result)
      }
    },
    onMutate: async (request) => {
      // Cancel outgoing refetches to avoid overwriting optimistic update
      await queryClient.cancelQueries({
        queryKey: queryKeys.games.snapshot(request.gameId),
      })

      // Snapshot current value for rollback
      const previousSnapshot =
        queryClient.getQueryData<GameRoomSnapshotPayload>(
          queryKeys.games.snapshot(request.gameId)
        )

      if (!previousSnapshot) {
        return { previousSnapshot: undefined }
      }

      // Skip optimistic update if not in Trick phase
      if (previousSnapshot.snapshot.phase.phase !== 'Trick') {
        return { previousSnapshot }
      }

      // Optimistically add card to current_trick
      const updatedSnapshot: GameRoomSnapshotPayload = {
        ...previousSnapshot,
        snapshot: {
          ...previousSnapshot.snapshot,
          phase: {
            phase: 'Trick',
            data: {
              ...previousSnapshot.snapshot.phase.data,
              current_trick: [
                ...previousSnapshot.snapshot.phase.data.current_trick,
                [previousSnapshot.snapshot.phase.data.to_act, request.card],
              ],
            },
          },
        },
      }

      queryClient.setQueryData(
        queryKeys.games.snapshot(request.gameId),
        updatedSnapshot
      )

      return { previousSnapshot }
    },
    onError: (err, request, context) => {
      // Rollback on error
      if (context?.previousSnapshot) {
        queryClient.setQueryData(
          queryKeys.games.snapshot(request.gameId),
          context.previousSnapshot
        )
      }
    },
    onSuccess: () => {
      // Invalidate last active game so header button updates
      queryClient.invalidateQueries({
        queryKey: queryKeys.games.waitingLongest(),
      })
    },
  })
}

/**
 * Mutation hook to add an AI seat.
 * Invalidates game snapshot cache on success.
 */
export function useAddAiSeat() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (request: ManageAiSeatRequest): Promise<void> => {
      const result = await addAiSeatAction(request)
      if (result.kind === 'error') {
        throw handleActionResultError(result)
      }
    },
    onSuccess: () => {
      // Invalidate AI registry since seat assignments changed
      queryClient.invalidateQueries({
        queryKey: queryKeys.ai.registry(),
      })
    },
  })
}

/**
 * Mutation hook to update an AI seat.
 * Invalidates game snapshot and AI registry cache on success.
 */
export function useUpdateAiSeat() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (request: ManageAiSeatRequest): Promise<void> => {
      const result = await updateAiSeatAction(request)
      if (result.kind === 'error') {
        throw handleActionResultError(result)
      }
    },
    onSuccess: () => {
      // Invalidate AI registry since seat assignments changed
      queryClient.invalidateQueries({
        queryKey: queryKeys.ai.registry(),
      })
    },
  })
}

/**
 * Mutation hook to remove an AI seat.
 * Invalidates game snapshot and AI registry cache on success.
 */
export function useRemoveAiSeat() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (request: ManageAiSeatRequest): Promise<void> => {
      const result = await removeAiSeatAction(request)
      if (result.kind === 'error') {
        throw handleActionResultError(result)
      }
    },
    onSuccess: () => {
      // Invalidate AI registry since seat assignments changed
      queryClient.invalidateQueries({
        queryKey: queryKeys.ai.registry(),
      })
    },
  })
}
