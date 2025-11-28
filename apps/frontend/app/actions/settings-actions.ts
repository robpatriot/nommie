'use server'

import { fetchWithAuth } from '@/lib/api'
import { toErrorResult } from '@/lib/api/action-helpers'
import type { SimpleActionResult } from '@/lib/api/action-helpers'
import type { ThemeMode } from '@/components/theme-provider'

export type UpdateUserOptionsPayload = {
  appearance_mode?: ThemeMode
  require_card_confirmation?: boolean
}

export async function updateUserOptionsAction(
  payload: UpdateUserOptionsPayload
): Promise<SimpleActionResult> {
  if (
    !payload ||
    (payload.appearance_mode === undefined &&
      payload.require_card_confirmation === undefined)
  ) {
    return {
      kind: 'error',
      message: 'No settings provided',
      status: 400,
      code: 'INVALID_SETTINGS_PAYLOAD',
    }
  }

  try {
    await fetchWithAuth('/api/user/options', {
      method: 'PUT',
      body: JSON.stringify(payload),
    })
    return { kind: 'ok' }
  } catch (error) {
    return toErrorResult(error, 'Failed to update settings')
  }
}

export async function updateAppearanceAction(mode: ThemeMode) {
  return updateUserOptionsAction({ appearance_mode: mode })
}
