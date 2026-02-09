'use client'

import { useMutation, useQueryClient } from '@tanstack/react-query'
import {
  createGameAction,
  joinGameAction,
  spectateGameAction,
  deleteGameAction,
} from '@/app/actions/game-actions'
import { handleActionResultError } from '@/lib/queries/query-error-handler'
import { queryKeys } from '@/lib/queries/query-keys'
import { requestLwRefetch } from '@/lib/queries/lw-cache'
import type { Game } from '@/lib/types'
import type { CreateGameRequest } from '@/app/actions/game-actions'

/**
 * Mutation hook to create a new game.
 * Invalidates games list cache on success.
 */
export function useCreateGame() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (request: CreateGameRequest): Promise<Game> => {
      const result = await createGameAction(request)
      if (result.kind === 'error') {
        throw handleActionResultError(result)
      }
      return result.data
    },
    // NOTE: Query invalidation removed - navigation happens immediately after mutateAsync
    // and we're leaving the lobby, so no need to refetch lobby queries.
    // The destination page will fetch fresh data if needed.
    onSuccess: () => {
      void requestLwRefetch(queryClient, { createSnapshot: false })
    },
  })
}

/**
 * Mutation hook to join a game.
 * Invalidates games list and specific game detail cache on success.
 */
export function useJoinGame() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (gameId: number): Promise<Game> => {
      const result = await joinGameAction(gameId)
      if (result.kind === 'error') {
        throw handleActionResultError(result)
      }
      return result.data
    },
    // NOTE: Query invalidation removed - navigation happens immediately after mutateAsync
    // and we're leaving the lobby, so no need to refetch lobby queries.
    // The destination page will fetch fresh data if needed.
    onSuccess: (_data, _gameId) => {
      void requestLwRefetch(queryClient, { createSnapshot: false })
    },
  })
}

/**
 * Mutation hook to spectate a game.
 * Invalidates games list and specific game detail cache on success.
 */
export function useSpectateGame() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (gameId: number): Promise<Game> => {
      const result = await spectateGameAction(gameId)
      if (result.kind === 'error') {
        throw handleActionResultError(result)
      }
      return result.data
    },
    // NOTE: Query invalidation removed - navigation happens immediately after mutateAsync
    // and we're leaving the lobby, so no need to refetch lobby queries.
    // The destination page will fetch fresh data if needed.
    onSuccess: (_data, _gameId) => {
      void requestLwRefetch(queryClient, { createSnapshot: false })
    },
  })
}

/**
 * Mutation hook to delete a game.
 * Invalidates games list and specific game detail cache on success.
 */
export function useDeleteGame() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({
      gameId,
      version,
    }: {
      gameId: number
      version?: number
    }): Promise<void> => {
      const result = await deleteGameAction(gameId, version)
      if (result.kind === 'error') {
        throw handleActionResultError(result)
      }
    },
    onSuccess: (_, { gameId }) => {
      // Invalidate games list and remove the specific game from cache
      void requestLwRefetch(queryClient, { createSnapshot: false })
      queryClient.invalidateQueries({ queryKey: queryKeys.games.listRoot() })
      queryClient.removeQueries({ queryKey: queryKeys.games.detail(gameId) })
    },
  })
}
