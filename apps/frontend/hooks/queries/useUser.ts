'use client'

import { useQuery } from '@tanstack/react-query'
import { getUserOptions } from '@/lib/api/user-options'
import { queryKeys } from '@/lib/queries/query-keys'
import type { UserOptionsResponse } from '@/lib/api/user-options'

/**
 * Query hook to fetch user options.
 * Uses the getUserOptions server function.
 */
export function useUserOptions() {
  return useQuery({
    queryKey: queryKeys.user.options(),
    queryFn: async (): Promise<UserOptionsResponse> => {
      return await getUserOptions()
    },
  })
}
