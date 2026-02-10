'use client'

import { useQuery } from '@tanstack/react-query'
import { useEffect } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import {
  refreshGamesListAction,
  getGameHistoryAction,
} from '@/app/actions/game-actions'
import { handleActionResultError } from '@/lib/queries/query-error-handler'
import { queryKeys } from '@/lib/queries/query-keys'
import type { Game } from '@/lib/types'
import type { GameHistoryApiResponse } from '@/app/actions/game-actions'
import {
  defaultLwCacheState,
  requestLwRefetch,
  type LwCacheState,
} from '@/lib/queries/lw-cache'

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
 * Query hook to fetch the list of game IDs that have been waiting longest.
 * Uses the getWaitingLongestGame server function.
 */
export function useWaitingLongestCache(options?: { enabled?: boolean }) {
  const queryClient = useQueryClient()
  const enabled = options?.enabled ?? true

  const q = useQuery<LwCacheState>({
    queryKey: queryKeys.games.waitingLongestCache(),
    queryFn: async (): Promise<LwCacheState> => defaultLwCacheState(),
    initialData: defaultLwCacheState,
    staleTime: Infinity,
    gcTime: Infinity,
    enabled,
  })

  useEffect(() => {
    if (!enabled) return
    const state = q.data
    if (!state) return
    if (state.refetchInFlight) return

    // Initial fetch: event-driven system still needs a first server answer for navigation.
    if (state.refetchRequestId === 0 && state.pool.length === 0) {
      void requestLwRefetch(queryClient, { createSnapshot: false })
    }
  }, [enabled, q.data, queryClient])

  return q
}

export function useWaitingLongestGame(options?: { enabled?: boolean }) {
  return useWaitingLongestCache(options)
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
    // History only changes when a round completes (or the game ends), so keep it
    // fresh indefinitely and invalidate explicitly on those events.
    staleTime: Infinity,
  })
}
