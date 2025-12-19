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
 */
export function useGameRoomSnapshot(
  gameId: number,
  options?: {
    etag?: string
    enabled?: boolean
    initialData?: GameRoomSnapshotPayload
  }
) {
  const queryClient = useQueryClient()

  return useQuery({
    queryKey: queryKeys.games.snapshot(gameId),
    queryFn: async (): Promise<GameRoomSnapshotPayload> => {
      const result = await getGameRoomSnapshotAction({
        gameId,
        etag: options?.etag,
      })

      if (result.kind === 'not_modified') {
        // Get the current cached data - this is what we want to return
        const cachedData = queryClient.getQueryData<GameRoomSnapshotPayload>(
          queryKeys.games.snapshot(gameId)
        )

        if (cachedData) {
          return cachedData
        }

        // If no cached data exists (shouldn't happen with ETag, but handle gracefully)
        // Fall back to initialData if provided
        if (options?.initialData) {
          return options.initialData
        }

        throw new Error('Not modified but no cached data available')
      }

      if (result.kind === 'error') {
        throw handleActionResultError(result)
      }

      return result.data
    },
    enabled: options?.enabled !== false && !!gameId,
    initialData: options?.initialData,
  })
}
