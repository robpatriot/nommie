'use server'

import { cookies } from 'next/headers'
import { fetchWithAuth } from '@/lib/api'
import { toErrorResult } from '@/lib/api/action-helpers'
import type { SimpleActionResult } from '@/lib/api/action-helpers'
import type { ThemeMode } from '@/components/theme-provider'
import {
  LOCALE_COOKIE_NAME,
  isSupportedLocale,
  type SupportedLocale,
} from '@/i18n/locale'

export type UpdateUserOptionsPayload = {
  appearance_mode?: ThemeMode
  require_card_confirmation?: boolean
  locale?: SupportedLocale
}

export async function updateUserOptionsAction(
  payload: UpdateUserOptionsPayload
): Promise<SimpleActionResult> {
  if (
    !payload ||
    (payload.appearance_mode === undefined &&
      payload.require_card_confirmation === undefined &&
      payload.locale === undefined)
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

    // If locale was updated, sync the cookie
    if (payload.locale && isSupportedLocale(payload.locale)) {
      const cookieStore = await cookies()
      cookieStore.set(LOCALE_COOKIE_NAME, payload.locale, {
        httpOnly: false,
        sameSite: 'lax',
        secure: process.env.NODE_ENV === 'production',
        path: '/',
        maxAge: 60 * 60 * 24 * 365,
      })
    }

    return { kind: 'ok' }
  } catch (error) {
    return toErrorResult(error, 'Failed to update settings')
  }
}

export async function updateAppearanceAction(mode: ThemeMode) {
  return updateUserOptionsAction({ appearance_mode: mode })
}

export async function updateLocaleAction(
  locale: SupportedLocale
): Promise<SimpleActionResult> {
  return updateUserOptionsAction({ locale })
}
