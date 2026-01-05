/**
 * Helper functions for working with game snapshot query data.
 */

import type { QueryClient } from '@tanstack/react-query'
import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
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
  const cachedSnapshot = queryClient.getQueryData<GameRoomSnapshotPayload>(
    queryKeys.games.snapshot(gameId)
  )
  return cachedSnapshot?.version
}
