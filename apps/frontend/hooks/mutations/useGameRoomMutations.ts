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
import {
  type GameRoomState,
  isBiddingPhase,
  isTrumpSelectPhase,
  isTrickPhase,
  selectSnapshot,
  selectViewerSeat,
} from '@/lib/game-room/state'
import { onOptimisticSend, setLwPendingAction } from '@/lib/queries/lw-cache'

/**
 * Mutation hook to set player ready status.
 * Invalidates game state cache on success.
 */
export function useMarkPlayerReady() {
  return useMutation({
    mutationFn: async ({
      gameId,
      isReady,
      version,
    }: {
      gameId: number
      isReady: boolean
      version: number
    }): Promise<void> => {
      const result = await markPlayerReadyAction(gameId, isReady, version)
      if (result.kind === 'error') {
        throw handleActionResultError(result)
      }
    },
    onSuccess: () => {},
  })
}

/**
 * Mutation hook to leave a game.
 * Invalidates game state cache on success and navigates to lobby.
 */
export function useLeaveGame() {
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
    onSuccess: () => {},
  })
}

/**
 * Mutation hook to rejoin a game.
 * Invalidates game state cache on success and navigates to game room.
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
      queryClient.invalidateQueries({
        queryKey: queryKeys.games.state(request.gameId),
      })
      router.push(`/game/${request.gameId}`)
    },
  })
}

/**
 * Mutation hook to submit a bid.
 * Uses optimistic updates to immediately show the bid in the UI.
 * Invalidates game state cache on success.
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
      onOptimisticSend(queryClient, { gameId: request.gameId })

      await queryClient.cancelQueries({
        queryKey: queryKeys.games.state(request.gameId),
      })

      const previousState = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(request.gameId)
      )

      if (!previousState) {
        return { previousState: undefined }
      }

      const viewerSeat = selectViewerSeat(previousState)
      const snapshot = selectSnapshot(previousState)
      const phase = snapshot.phase
      if (viewerSeat === null || !isBiddingPhase(phase)) {
        return { previousState }
      }

      const phaseData = phase.data
      const updatedState: GameRoomState = {
        ...previousState,
        source: 'optimistic',
        game: {
          ...snapshot,
          phase: {
            phase: 'Bidding',
            data: {
              ...phaseData,
              bids: phaseData.bids.map((bid, idx) =>
                idx === viewerSeat ? request.bid : bid
              ) as [number | null, number | null, number | null, number | null],
              round: {
                ...phaseData.round,
                bids: phaseData.round.bids.map((bid, idx) =>
                  idx === viewerSeat ? request.bid : bid
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
        queryKeys.games.state(request.gameId),
        updatedState
      )

      return { previousState }
    },
    onError: (err, request, context) => {
      setLwPendingAction(queryClient, request.gameId, false)
      if (context?.previousState) {
        queryClient.setQueryData(
          queryKeys.games.state(request.gameId),
          context.previousState
        )
      }
    },
    onSuccess: () => {
      // LW navigation is refreshed via realtime events + LW cache module.
    },
  })
}

/**
 * Mutation hook to select trump suit.
 * Uses optimistic updates to immediately show the trump suit in the UI.
 * Invalidates game state cache on success.
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
      onOptimisticSend(queryClient, { gameId: request.gameId })

      await queryClient.cancelQueries({
        queryKey: queryKeys.games.state(request.gameId),
      })

      const previousState = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(request.gameId)
      )

      if (!previousState) {
        return { previousState: undefined }
      }

      const snapshot = selectSnapshot(previousState)
      const phase = snapshot.phase
      if (!isTrumpSelectPhase(phase)) {
        return { previousState }
      }

      const phaseData = phase.data
      const updatedState: GameRoomState = {
        ...previousState,
        source: 'optimistic',
        game: {
          ...snapshot,
          phase: {
            phase: 'TrumpSelect',
            data: {
              ...phaseData,
              round: {
                ...phaseData.round,
                trump: request.trump,
              },
            },
          },
        },
      }

      queryClient.setQueryData(
        queryKeys.games.state(request.gameId),
        updatedState
      )

      return { previousState }
    },
    onError: (err, request, context) => {
      setLwPendingAction(queryClient, request.gameId, false)
      if (context?.previousState) {
        queryClient.setQueryData(
          queryKeys.games.state(request.gameId),
          context.previousState
        )
      }
    },
    onSuccess: () => {
      // LW navigation is refreshed via realtime events + LW cache module.
    },
  })
}

/**
 * Mutation hook to submit a card play.
 * Uses optimistic updates to immediately show the card in the trick.
 * Invalidates game state cache on success.
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
      onOptimisticSend(queryClient, { gameId: request.gameId })

      await queryClient.cancelQueries({
        queryKey: queryKeys.games.state(request.gameId),
      })

      const previousState = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(request.gameId)
      )

      if (!previousState) {
        return { previousState: undefined }
      }

      const snapshot = selectSnapshot(previousState)
      const phase = snapshot.phase
      if (!isTrickPhase(phase)) {
        return { previousState }
      }

      const phaseData = phase.data
      const updatedState: GameRoomState = {
        ...previousState,
        source: 'optimistic',
        game: {
          ...snapshot,
          phase: {
            phase: 'Trick',
            data: {
              ...phaseData,
              current_trick: [
                ...phaseData.current_trick,
                [phaseData.to_act, request.card],
              ],
            },
          },
        },
      }

      queryClient.setQueryData(
        queryKeys.games.state(request.gameId),
        updatedState
      )

      return { previousState }
    },
    onError: (err, request, context) => {
      setLwPendingAction(queryClient, request.gameId, false)
      if (context?.previousState) {
        queryClient.setQueryData(
          queryKeys.games.state(request.gameId),
          context.previousState
        )
      }
    },
    onSuccess: () => {
      // LW navigation is refreshed via realtime events + LW cache module.
    },
  })
}

/**
 * Mutation hook to add an AI seat.
 * Invalidates game state cache on success.
 */
export function useAddAiSeat() {
  return useMutation({
    mutationFn: async (request: ManageAiSeatRequest): Promise<void> => {
      const result = await addAiSeatAction(request)
      if (result.kind === 'error') {
        throw handleActionResultError(result)
      }
    },
  })
}

/**
 * Mutation hook to update an AI seat.
 * Invalidates game state and AI registry cache on success.
 */
export function useUpdateAiSeat() {
  return useMutation({
    mutationFn: async (request: ManageAiSeatRequest): Promise<void> => {
      const result = await updateAiSeatAction(request)
      if (result.kind === 'error') {
        throw handleActionResultError(result)
      }
    },
  })
}

/**
 * Mutation hook to remove an AI seat.
 * Invalidates game state and AI registry cache on success.
 */
export function useRemoveAiSeat() {
  return useMutation({
    mutationFn: async (request: ManageAiSeatRequest): Promise<void> => {
      const result = await removeAiSeatAction(request)
      if (result.kind === 'error') {
        throw handleActionResultError(result)
      }
    },
  })
}
