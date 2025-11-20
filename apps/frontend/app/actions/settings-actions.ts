'use server'

import { BackendApiError, fetchWithAuth } from '@/lib/api'
import type { ThemeMode } from '@/components/theme-provider'

export type UpdateAppearanceResult =
  | { success: true; error?: never }
  | { error: BackendApiError; success?: never }

export async function updateAppearanceAction(
  mode: ThemeMode
): Promise<UpdateAppearanceResult> {
  try {
    await fetchWithAuth('/api/user/options', {
      method: 'PUT',
      body: JSON.stringify({ appearance_mode: mode }),
    })
    return { success: true }
  } catch (error) {
    if (error instanceof BackendApiError) {
      return { error }
    }
    return {
      error: new BackendApiError(
        error instanceof Error ? error.message : 'Failed to update appearance',
        500,
        'UNKNOWN_ERROR'
      ),
    }
  }
}
