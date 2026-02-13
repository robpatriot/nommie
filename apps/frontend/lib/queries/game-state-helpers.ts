/**
 * Helper functions for working with game room state query data.
 */

import type { QueryClient } from '@tanstack/react-query'
import type { GameRoomState } from '@/lib/game-room/state'
import { queryKeys } from './query-keys'

/**
 * Get the current game version from the query cache.
 * Reads the version directly from cache at request time to avoid stale closures.
 *
 * @param queryClient - TanStack Query client
 * @param gameId - Game ID
 * @returns The game version, or undefined if not available
 */
export function getGameVersionFromCache(
  queryClient: QueryClient,
  gameId: number
): number | undefined {
  const cachedState = queryClient.getQueryData<GameRoomState>(
    queryKeys.games.state(gameId)
  )
  return cachedState?.version
}
