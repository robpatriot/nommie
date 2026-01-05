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
    onSuccess: () => {
      // Invalidate games list so it refreshes with the new game
      queryClient.invalidateQueries({ queryKey: queryKeys.games.lists() })
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
    onSuccess: (data, gameId) => {
      // Invalidate games list and the specific game detail
      queryClient.invalidateQueries({ queryKey: queryKeys.games.lists() })
      queryClient.invalidateQueries({
        queryKey: queryKeys.games.detail(gameId),
      })
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
    onSuccess: (data, gameId) => {
      // Invalidate games list and the specific game detail
      queryClient.invalidateQueries({ queryKey: queryKeys.games.lists() })
      queryClient.invalidateQueries({
        queryKey: queryKeys.games.detail(gameId),
      })
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
      queryClient.invalidateQueries({ queryKey: queryKeys.games.lists() })
      queryClient.removeQueries({ queryKey: queryKeys.games.detail(gameId) })
    },
  })
}
