'use server'

import { cookies } from 'next/headers'
import { getTranslations } from 'next-intl/server'
import { fetchWithAuth } from '@/lib/api'
import { toErrorResult } from '@/lib/api/action-helpers'
import type { SimpleActionResult } from '@/lib/api/action-helpers'
import type { ColourScheme, ThemeName } from '@/components/theme-provider'
import {
  LOCALE_COOKIE_NAME,
  isSupportedLocale,
  type SupportedLocale,
} from '@/i18n/locale'

export type UpdateUserOptionsPayload = {
  colour_scheme?: ColourScheme
  theme?: ThemeName
  require_card_confirmation?: boolean
  locale?: SupportedLocale | null
  trick_display_duration_seconds?: number | null
}

export async function updateUserOptionsAction(
  payload: UpdateUserOptionsPayload
): Promise<SimpleActionResult> {
  if (
    !payload ||
    (payload.colour_scheme === undefined &&
      payload.theme === undefined &&
      payload.require_card_confirmation === undefined &&
      payload.locale === undefined &&
      payload.trick_display_duration_seconds === undefined)
  ) {
    // This is a validation error that should be localized
    const t = await getTranslations('errors.validation')
    return {
      kind: 'error',
      message: t('noSettingsProvided'),
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
    if (payload.locale !== undefined) {
      const cookieStore = await cookies()
      if (payload.locale && isSupportedLocale(payload.locale)) {
        // Set cookie to match backend preference
        cookieStore.set(LOCALE_COOKIE_NAME, payload.locale, {
          httpOnly: false,
          sameSite: 'lax',
          secure: process.env.NODE_ENV === 'production',
          path: '/',
          maxAge: 60 * 60 * 24 * 365,
        })
      } else {
        // Unset preference - delete the cookie so browser setting takes precedence
        cookieStore.delete(LOCALE_COOKIE_NAME)
      }
    }

    return { kind: 'ok' }
  } catch (error) {
    const t = await getTranslations('errors.actions')
    return toErrorResult(error, t('failedToUpdateSettings'))
  }
}

export async function updateLocaleAction(
  locale: SupportedLocale | null
): Promise<SimpleActionResult> {
  return updateUserOptionsAction({ locale })
}
