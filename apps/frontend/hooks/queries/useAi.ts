'use client'

import { useQuery } from '@tanstack/react-query'
import { fetchAiRegistryAction } from '@/app/actions/game-room-actions'
import { handleActionResultError } from '@/lib/queries/query-error-handler'
import { queryKeys } from '@/lib/queries/query-keys'
import type { AiRegistryEntry } from '@/lib/api/game-room'

/**
 * Query hook to fetch AI registry.
 * Uses the fetchAiRegistryAction server action.
 * @param enabled - Whether the query should be enabled (default: true)
 */
export function useAiRegistry(enabled: boolean = true) {
  return useQuery({
    queryKey: queryKeys.ai.registry(),
    queryFn: async (): Promise<AiRegistryEntry[]> => {
      const result = await fetchAiRegistryAction()
      if (result.kind === 'error') {
        throw handleActionResultError(result)
      }
      return result.data
    },
    enabled,
  })
}
