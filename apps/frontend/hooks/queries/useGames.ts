'use client'

import { useQuery } from '@tanstack/react-query'
import {
  refreshGamesListAction,
  getGameHistoryAction,
} from '@/app/actions/game-actions'
import { getWaitingLongestGame } from '@/lib/api'
import {
  handleActionResultError,
  toQueryError,
} from '@/lib/queries/query-error-handler'
import { queryKeys } from '@/lib/queries/query-keys'
import type { Game } from '@/lib/types'
import type { GameHistoryApiResponse } from '@/app/actions/game-actions'

/**
 * Query hook to fetch available games (joinable and in-progress).
 * Uses the refreshGamesListAction server action.
 * @param initialData - Optional initial data from server component
 */
export function useAvailableGames(initialData?: Game[]) {
  return useQuery({
    queryKey: queryKeys.games.listRoot(),
    queryFn: async () => {
      const result = await refreshGamesListAction()
      if (result.kind === 'error') {
        throw handleActionResultError(result)
      }
      return result.data
    },
    initialData,
    // Data is considered stale immediately so it refetches in background
    staleTime: 0,
  })
}

/**
 * Query hook to fetch the game ID that has been waiting the longest.
 * Uses the getWaitingLongestGame server function.
 * @param options.excludeGameId - Optional ID of a game to exclude from the result
 */
export function useWaitingLongestGame(options?: {
  excludeGameId?: number
  enabled?: boolean
}) {
  return useQuery({
    queryKey: queryKeys.games.waitingLongest(options?.excludeGameId),
    queryFn: async () => {
      try {
        return await getWaitingLongestGame(options?.excludeGameId)
      } catch (error) {
        throw toQueryError(error, 'Failed to fetch waiting game')
      }
    },
    staleTime: Infinity,
    enabled: options?.enabled ?? true,
  })
}

/**
 * Query hook to fetch game history.
 * Uses the getGameHistoryAction server action.
 */
export function useGameHistory(gameId: number | undefined) {
  return useQuery({
    queryKey: gameId
      ? queryKeys.games.history(gameId)
      : ['games', 'history', 'disabled'],
    queryFn: async (): Promise<GameHistoryApiResponse> => {
      if (!gameId) {
        throw new Error('Game ID is required')
      }
      const result = await getGameHistoryAction(gameId)
      if (result.kind === 'error') {
        throw handleActionResultError(result)
      }
      return result.data
    },
    enabled: !!gameId,
    // 30 seconds - changes after each round completes
    staleTime: 30 * 1000,
  })
}
