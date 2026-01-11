'use client'

import { useMutation, useQueryClient } from '@tanstack/react-query'
import {
  updateUserOptionsAction,
  type UpdateUserOptionsPayload,
} from '@/app/actions/settings-actions'
import { handleActionResultError } from '@/lib/queries/query-error-handler'
import { queryKeys } from '@/lib/queries/query-keys'

/**
 * Mutation hook to update user options.
 * Invalidates user options cache on success.
 */
export function useUpdateUserOptions() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (payload: UpdateUserOptionsPayload): Promise<void> => {
      const result = await updateUserOptionsAction(payload)
      if (result.kind === 'error') {
        throw handleActionResultError(result)
      }
    },
    onSuccess: () => {
      // Invalidate user options cache so it refreshes with updated settings
      queryClient.invalidateQueries({ queryKey: queryKeys.user.options() })
    },
  })
}
