'use client'

import { useQuery, useQueryClient } from '@tanstack/react-query'
import { getGameRoomSnapshotAction } from '@/app/actions/game-room-actions'
import { handleActionResultError } from '@/lib/queries/query-error-handler'
import { queryKeys } from '@/lib/queries/query-keys'
import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'

/**
 * Query hook to fetch and transform game room snapshot.
 * Uses the getGameRoomSnapshotAction server action which handles transformation.
 * Handles the 'not_modified' case (304 response) by returning the cached data directly.
 * Disables refetch on window focus since WebSocket handles real-time updates.
 *
 * **ETag handling**: The `etag` option is used for HTTP conditional requests
 * (If-None-Match header) but is NOT included in the query key. This ensures
 * a single cache entry per game, which is updated via WebSocket messages or
 * manual refresh calls. ETag changes do not trigger automatic refetches - the
 * system relies on WebSocket updates or explicit refresh calls for real-time
 * synchronization.
 */
export function useGameSnapshot(
  gameId: number,
  options?: {
    etag?: string
    enabled?: boolean
    initialData?: GameRoomSnapshotPayload
  }
) {
  const queryClient = useQueryClient()

  // Reconcile server-provided initialData into the TanStack cache (upgrade-only).
  // This prevents stale cache from winning on SPA navigation.
  const incoming = options?.initialData
  if (incoming?.version !== undefined) {
    const key = queryKeys.games.snapshot(gameId)
    const cached = queryClient.getQueryData<GameRoomSnapshotPayload>(key)

    const cachedVersion = cached?.version
    const shouldUpgrade =
      cached == null ||
      cachedVersion === undefined ||
      incoming.version > cachedVersion

    if (shouldUpgrade) {
      queryClient.setQueryData(key, incoming)
    }
  }

  return useQuery({
    queryKey: queryKeys.games.snapshot(gameId),
    queryFn: async (): Promise<GameRoomSnapshotPayload> => {
      const result = await getGameRoomSnapshotAction({
        gameId,
        etag: options?.etag,
      })

      if (result.kind === 'not_modified') {
        // 304 means we already have the current data
        const cachedData = queryClient.getQueryData<GameRoomSnapshotPayload>(
          queryKeys.games.snapshot(gameId)
        )

        if (cachedData) {
          return cachedData
        }

        // If we got 304 but have no cache, use initialData if available
        // This handles the edge case where cache was cleared between mount and query
        if (options?.initialData) {
          return options.initialData
        }

        // This shouldn't happen: we sent an ETag but have no cached data
        // Return a clear error rather than making another network request
        throw new Error(
          'Received 304 Not Modified but no cached data available'
        )
      }

      if (result.kind === 'error') {
        throw handleActionResultError(result)
      }

      return result.data
    },
    enabled: options?.enabled !== false && !!gameId,
    initialData: options?.initialData,
    // 5 seconds - real-time game data changes frequently, WebSocket handles most updates
    staleTime: 5 * 1000,
    // Disable refetch on window focus since WebSocket handles real-time updates
    refetchOnWindowFocus: false,
  })
}
