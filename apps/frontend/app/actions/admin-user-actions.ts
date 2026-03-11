'use server'

import {
  searchAdminUsers,
  grantAdmin,
  revokeAdmin,
  type AdminUserSearchResponse,
  type RoleMutationResponse,
} from '@/lib/api/admin-users'
import { toErrorResult } from '@/lib/api/action-helpers'
import type { ActionResult } from '@/lib/api/action-helpers'

export async function searchAdminUsersAction(params: {
  q: string
  limit?: number
  cursor?: string | null
}): Promise<ActionResult<AdminUserSearchResponse>> {
  try {
    const data = await searchAdminUsers(params)
    return { kind: 'ok', data }
  } catch (error) {
    return toErrorResult(error, 'Failed to search users')
  }
}

export async function grantAdminAction(
  userId: number,
  body?: { reason?: string | null }
): Promise<ActionResult<RoleMutationResponse>> {
  try {
    const data = await grantAdmin(userId, body)
    return { kind: 'ok', data }
  } catch (error) {
    return toErrorResult(error, 'Failed to grant admin')
  }
}

export async function revokeAdminAction(
  userId: number,
  body?: { reason?: string | null }
): Promise<ActionResult<RoleMutationResponse>> {
  try {
    const data = await revokeAdmin(userId, body)
    return { kind: 'ok', data }
  } catch (error) {
    return toErrorResult(error, 'Failed to revoke admin')
  }
}
