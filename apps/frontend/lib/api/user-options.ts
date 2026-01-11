'use server'

import { fetchWithAuth } from '@/lib/api'
import type { ColourScheme, ThemeName } from '@/components/theme-provider'
import type { SupportedLocale } from '@/i18n/locale'

export interface UserOptionsResponse {
  colour_scheme: ColourScheme
  theme: ThemeName
  require_card_confirmation: boolean
  locale: SupportedLocale | null
  trick_display_duration_seconds: number | null
  updated_at: string
}

export async function getUserOptions(): Promise<UserOptionsResponse> {
  const response = await fetchWithAuth('/api/user/options')
  const data = (await response.json()) as UserOptionsResponse
  return data
}
