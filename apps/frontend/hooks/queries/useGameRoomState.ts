'use client'

import { useEffect } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { getGameRoomStateAction } from '@/app/actions/game-room-actions'
import { handleActionResultError } from '@/lib/queries/query-error-handler'
import { queryKeys } from '@/lib/queries/query-keys'
import type { GameRoomState } from '@/lib/game-room/state'

/**
 * Query hook to fetch and cache game room state.
 * Uses GameRoomState as the cache model; WebSocket updates overwrite via useGameSync.
 * Handles the 'not_modified' case (304 response) by returning cached data.
 */
export function useGameRoomState(
  gameId: number,
  options?: {
    etag?: string
    enabled?: boolean
    initialData?: GameRoomState
  }
) {
  const queryClient = useQueryClient()
  const incomingVersion = options?.initialData?.version

  useEffect(() => {
    const incoming = options?.initialData
    if (incoming?.version === undefined) return

    const key = queryKeys.games.state(gameId)
    const cached = queryClient.getQueryData<GameRoomState>(key)
    const cachedVersion = cached?.version
    const shouldUpgrade =
      cached == null ||
      cachedVersion === undefined ||
      incoming.version > cachedVersion

    if (shouldUpgrade) {
      queryClient.setQueryData(key, incoming)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- incomingVersion is stable signal; options?.initialData read inside
  }, [gameId, incomingVersion, queryClient])

  return useQuery({
    queryKey: queryKeys.games.state(gameId),
    queryFn: async (): Promise<GameRoomState> => {
      const result = await getGameRoomStateAction({
        gameId,
        etag: options?.etag,
      })

      if (result.kind === 'not_modified') {
        const cachedData = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(gameId)
        )

        if (cachedData) {
          return cachedData
        }

        if (options?.initialData) {
          return options.initialData
        }

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
    staleTime: Infinity,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
  })
}
