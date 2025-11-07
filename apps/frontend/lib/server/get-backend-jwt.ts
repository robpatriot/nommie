// Server-only helper to get backend JWT from NextAuth token
// This file must never be imported by client code

import { auth } from '@/auth'
import { getToken } from 'next-auth/jwt'
import { cookies } from 'next/headers'
import { redirect } from 'next/navigation'
import type { Session } from 'next-auth'

export type BackendJwtResolution =
  | { state: 'missing-session'; session: null }
  | { state: 'missing-jwt'; session: Session }
  | { state: 'ready'; session: Session; backendJwt: string }

export function isAuthDisabled(): boolean {
  return process.env.NEXT_PUBLIC_DISABLE_AUTH === 'true'
}

export async function resolveBackendJwt(): Promise<BackendJwtResolution> {
  const session = await auth()

  if (!session) {
    return { state: 'missing-session', session: null }
  }

  const cookieStore = await cookies()
  const cookieHeader = cookieStore
    .getAll()
    .map(({ name, value }) => `${name}=${value}`)
    .join('; ')

  if (!cookieHeader) {
    return { state: 'missing-jwt', session }
  }

  const headers = new Headers({ cookie: cookieHeader })
  const req: { headers: Headers } = { headers }

  try {
    const token = await getToken({ req, secret: process.env.AUTH_SECRET })
    const backendJwt = token?.backendJwt
    if (typeof backendJwt === 'string' && backendJwt.length > 0) {
      return { state: 'ready', session, backendJwt }
    }
  } catch {
    // If decoding fails, treat it as a missing JWT so callers can handle re-auth.
  }

  return { state: 'missing-jwt', session }
}

export async function requireBackendJwt(): Promise<string> {
  if (isAuthDisabled()) {
    throw new Error(
      'requireBackendJwt should not be called when auth is disabled'
    )
  }

  const resolution = await resolveBackendJwt()

  switch (resolution.state) {
    case 'ready':
      return resolution.backendJwt
    case 'missing-session':
      redirect('/')
    case 'missing-jwt':
      redirect('/api/auth/signout?callbackUrl=%2F')
  }
}
