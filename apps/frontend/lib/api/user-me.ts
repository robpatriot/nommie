'use server'

import { fetchWithAuth } from '@/lib/api'
import { BackendApiError } from '@/lib/errors'

export type MeResponse = {
  id: number
  username: string | null
  role: 'user' | 'admin'
}

/**
 * Fetches the current user from GET /api/user/me.
 * Returns null on 401 (unauthenticated) — normal unauthenticated state.
 * Throws on other errors.
 */
export async function getMe(): Promise<MeResponse | null> {
  try {
    const response = await fetchWithAuth('/api/user/me')
    const data = (await response.json()) as MeResponse
    return data
  } catch (error) {
    if (error instanceof BackendApiError && error.status === 401) {
      return null
    }
    throw error
  }
}
