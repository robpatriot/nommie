'use server'

import { BackendApiError, fetchWithAuth } from '@/lib/api'
import type { ThemeMode } from '@/components/theme-provider'

export type UpdateUserOptionsResult =
  | { success: true; error?: never }
  | { error: BackendApiError; success?: never }

export type UpdateUserOptionsPayload = {
  appearance_mode?: ThemeMode
  require_card_confirmation?: boolean
}

export async function updateUserOptionsAction(
  payload: UpdateUserOptionsPayload
): Promise<UpdateUserOptionsResult> {
  if (
    !payload ||
    (payload.appearance_mode === undefined &&
      payload.require_card_confirmation === undefined)
  ) {
    return {
      error: new BackendApiError(
        'No settings provided',
        400,
        'INVALID_SETTINGS_PAYLOAD'
      ),
    }
  }

  try {
    await fetchWithAuth('/api/user/options', {
      method: 'PUT',
      body: JSON.stringify(payload),
    })
    return { success: true }
  } catch (error) {
    if (error instanceof BackendApiError) {
      return { error }
    }
    return {
      error: new BackendApiError(
        error instanceof Error ? error.message : 'Failed to update settings',
        500,
        'UNKNOWN_ERROR'
      ),
    }
  }
}

export async function updateAppearanceAction(mode: ThemeMode) {
  return updateUserOptionsAction({ appearance_mode: mode })
}
