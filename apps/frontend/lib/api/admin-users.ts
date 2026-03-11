'use server'

import { fetchWithAuth } from '@/lib/api'

export type AdminUserSummary = {
  id: number
  display_name: string | null
  email: string | null
  role: 'user' | 'admin'
}

export type AdminUserSearchResponse = {
  items: AdminUserSummary[]
  next_cursor: string | null
}

export type RoleMutationResponse = {
  user: AdminUserSummary
  changed: boolean
}

export type RoleMutationRequest = {
  reason?: string | null
}

/**
 * Search users for admin role management.
 * GET /api/admin/users/search?q=...&limit=...&cursor=...
 */
export async function searchAdminUsers(params: {
  q: string
  limit?: number
  cursor?: string | null
}): Promise<AdminUserSearchResponse> {
  const { q, limit = 20, cursor } = params
  const searchParams = new URLSearchParams()
  searchParams.set('q', q.trim())
  if (limit > 0) searchParams.set('limit', String(limit))
  if (cursor) searchParams.set('cursor', cursor)

  const response = await fetchWithAuth(
    `/api/admin/users/search?${searchParams.toString()}`
  )
  const data = (await response.json()) as AdminUserSearchResponse
  return data
}

/**
 * Grant admin role to a user.
 * POST /api/admin/users/{userId}/grant-admin
 */
export async function grantAdmin(
  userId: number,
  body?: RoleMutationRequest
): Promise<RoleMutationResponse> {
  const response = await fetchWithAuth(
    `/api/admin/users/${userId}/grant-admin`,
    {
      method: 'POST',
      body: JSON.stringify(body ?? {}),
    }
  )
  const data = (await response.json()) as RoleMutationResponse
  return data
}

/**
 * Revoke admin role from a user.
 * POST /api/admin/users/{userId}/revoke-admin
 */
export async function revokeAdmin(
  userId: number,
  body?: RoleMutationRequest
): Promise<RoleMutationResponse> {
  const response = await fetchWithAuth(
    `/api/admin/users/${userId}/revoke-admin`,
    {
      method: 'POST',
      body: JSON.stringify(body ?? {}),
    }
  )
  const data = (await response.json()) as RoleMutationResponse
  return data
}
