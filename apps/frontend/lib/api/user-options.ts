'use server'

import { fetchWithAuth } from '@/lib/api'
import type { ThemeMode } from '@/components/theme-provider'

export interface UserOptionsResponse {
  appearance_mode: ThemeMode
  require_card_confirmation: boolean
  updated_at: string
}

export async function getUserOptions(): Promise<UserOptionsResponse> {
  const response = await fetchWithAuth('/api/user/options')
  const data = (await response.json()) as UserOptionsResponse
  return data
}
