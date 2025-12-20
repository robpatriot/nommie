'use client'

import { useQuery } from '@tanstack/react-query'
import {
  refreshGamesListAction,
  getGameHistoryAction,
} from '@/app/actions/game-actions'
import { fetchGameSnapshot } from '@/lib/api/game-room'
import { getLastActiveGame } from '@/lib/api'
import { handleActionResultError } from '@/lib/queries/query-error-handler'
import { queryKeys } from '@/lib/queries/query-keys'
import type { Game } from '@/lib/types'
import type { GameSnapshotResult } from '@/lib/api/game-room'
import type { GameHistoryApiResponse } from '@/app/actions/game-actions'

/**
 * Query hook to fetch available games (joinable and in-progress).
 * Uses the refreshGamesListAction server action.
 * @param initialData - Optional initial data from server component
 */
export function useAvailableGames(initialData?: Game[]) {
  return useQuery({
    queryKey: queryKeys.games.lists(),
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
 * Query hook to fetch the last active game ID.
 * Uses the getLastActiveGame server function.
 * Errors are handled consistently through toQueryError.
 */
export function useLastActiveGame() {
  return useQuery({
    queryKey: queryKeys.games.lastActive(),
    queryFn: async () => {
      try {
        return await getLastActiveGame()
      } catch (error) {
        // Ensure consistent error handling - fetchWithAuth throws BackendApiError,
        // but wrap in toQueryError for consistency with other queries
        throw toQueryError(error, 'Failed to fetch last active game')
      }
    },
  })
}

/**
 * Query hook to fetch game snapshot with ETag support.
 * Uses the fetchGameSnapshot server function.
 */
export function useGameSnapshot(
  gameId: number,
  options?: { etag?: string; enabled?: boolean }
) {
  return useQuery({
    queryKey: queryKeys.games.snapshot(gameId),
    queryFn: async (): Promise<GameSnapshotResult> => {
      return await fetchGameSnapshot(gameId, { etag: options?.etag })
    },
    enabled: options?.enabled !== false && !!gameId,
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
  })
}
