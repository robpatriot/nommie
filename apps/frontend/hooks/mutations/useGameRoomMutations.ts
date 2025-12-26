'use client'

import { useMutation, useQueryClient } from '@tanstack/react-query'
import {
  markPlayerReadyAction,
  leaveGameAction,
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
    onSuccess: (_, { gameId }) => {
      // Invalidate game snapshot so it refreshes with updated state
      queryClient.invalidateQueries({
        queryKey: queryKeys.games.snapshot(gameId),
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
    onSuccess: (_, request) => {
      // Invalidate game snapshot so it refreshes with updated state
      queryClient.invalidateQueries({
        queryKey: queryKeys.games.snapshot(request.gameId),
      })
    },
  })
}

/**
 * Mutation hook to submit a bid.
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
    onSuccess: (_, request) => {
      // Invalidate game snapshot so it refreshes with updated state
      queryClient.invalidateQueries({
        queryKey: queryKeys.games.snapshot(request.gameId),
      })
    },
  })
}

/**
 * Mutation hook to select trump suit.
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
    onSuccess: (_, request) => {
      // Invalidate game snapshot so it refreshes with updated state
      queryClient.invalidateQueries({
        queryKey: queryKeys.games.snapshot(request.gameId),
      })
    },
  })
}

/**
 * Mutation hook to submit a card play.
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
    onSuccess: (_, request) => {
      // Invalidate game snapshot so it refreshes with updated state
      queryClient.invalidateQueries({
        queryKey: queryKeys.games.snapshot(request.gameId),
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
    onSuccess: (_, request) => {
      // Invalidate game snapshot so it refreshes with updated state
      queryClient.invalidateQueries({
        queryKey: queryKeys.games.snapshot(request.gameId),
      })
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
    onSuccess: (_, request) => {
      // Invalidate game snapshot so it refreshes with updated state
      queryClient.invalidateQueries({
        queryKey: queryKeys.games.snapshot(request.gameId),
      })
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
    onSuccess: (_, request) => {
      // Invalidate game snapshot so it refreshes with updated state
      queryClient.invalidateQueries({
        queryKey: queryKeys.games.snapshot(request.gameId),
      })
      // Invalidate AI registry since seat assignments changed
      queryClient.invalidateQueries({
        queryKey: queryKeys.ai.registry(),
      })
    },
  })
}
