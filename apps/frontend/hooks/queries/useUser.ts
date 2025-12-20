'use client'

import { useQuery } from '@tanstack/react-query'
import { getUserOptions } from '@/lib/api/user-options'
import { toQueryError } from '@/lib/queries/query-error-handler'
import { queryKeys } from '@/lib/queries/query-keys'
import type { UserOptionsResponse } from '@/lib/api/user-options'

/**
 * Query hook to fetch user options.
 * Uses the getUserOptions server function.
 * Errors are handled consistently through toQueryError.
 */
export function useUserOptions() {
  return useQuery({
    queryKey: queryKeys.user.options(),
    queryFn: async (): Promise<UserOptionsResponse> => {
      try {
        return await getUserOptions()
      } catch (error) {
        // Ensure consistent error handling - fetchWithAuth throws BackendApiError,
        // but wrap in toQueryError for consistency with other queries
        throw toQueryError(error, 'Failed to fetch user options')
      }
    },
  })
}
