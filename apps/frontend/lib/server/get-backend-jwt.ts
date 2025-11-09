// Server-only helper to get backend JWT from NextAuth token
// This file must never be imported by client code

import { auth, getBackendBaseUrlOrThrow } from '@/auth'
import { getToken } from 'next-auth/jwt'
import { cookies } from 'next/headers'
import { redirect } from 'next/navigation'
import type { Session } from 'next-auth'

interface NextAuthToken {
  backendJwt?: unknown
  email?: unknown
  name?: unknown
  googleSub?: unknown
  sub?: unknown
}

const REFRESH_THRESHOLD_SECONDS = 60

export type BackendJwtResolution =
  | { state: 'missing-session'; session: null }
  | { state: 'missing-jwt'; session: Session }
  | { state: 'ready'; session: Session; backendJwt: string }

export function isAuthDisabled(): boolean {
  return process.env.NEXT_PUBLIC_DISABLE_AUTH === 'true'
}

export class BackendJwtMissingError extends Error {
  constructor(message = 'Backend JWT is missing') {
    super(message)
    this.name = 'BackendJwtMissingError'
  }
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
    const token = (await getToken({
      req,
      secret: process.env.AUTH_SECRET,
    })) as NextAuthToken | null

    const existingJwt =
      token?.backendJwt && typeof token.backendJwt === 'string'
        ? token.backendJwt
        : undefined

    if (existingJwt && !isJwtExpiring(existingJwt, REFRESH_THRESHOLD_SECONDS)) {
      return { state: 'ready', session, backendJwt: existingJwt }
    }

    const refreshed = await refreshBackendJwt({ session, token })
    if (refreshed) {
      return { state: 'ready', session, backendJwt: refreshed }
    }
  } catch (error) {
    console.error('Failed to resolve backend JWT', error)
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
      throw new BackendJwtMissingError()
  }
}

function isJwtExpiring(token: string, thresholdSeconds: number): boolean {
  try {
    const [, payload] = token.split('.')
    if (!payload) return true

    const decoded = JSON.parse(
      Buffer.from(payload, 'base64url').toString()
    ) as {
      exp?: number
    }
    if (typeof decoded.exp !== 'number') {
      return true
    }

    const nowSeconds = Math.floor(Date.now() / 1000)
    return decoded.exp - nowSeconds <= thresholdSeconds
  } catch {
    return true
  }
}

async function refreshBackendJwt({
  session,
  token,
}: {
  session: Session
  token: NextAuthToken | null
}): Promise<string | null> {
  const email =
    (typeof session.user?.email === 'string' && session.user.email) ||
    (token?.email && typeof token.email === 'string' ? token.email : null)
  const googleSub =
    (token?.googleSub &&
      typeof token.googleSub === 'string' &&
      token.googleSub) ||
    (token?.sub && typeof token.sub === 'string' ? token.sub : null)
  const name =
    typeof session.user?.name === 'string'
      ? session.user.name
      : typeof token?.name === 'string'
        ? (token.name as string)
        : undefined

  if (!email || !googleSub) {
    console.warn('Unable to refresh backend JWT: missing email or googleSub')
    return null
  }

  const backendBase = getBackendBaseUrlOrThrow()

  try {
    const response = await fetch(`${backendBase}/api/auth/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        email,
        name,
        google_sub: googleSub,
      }),
    })

    if (!response.ok) {
      console.warn('Backend JWT refresh failed', response.status)
      return null
    }

    const data = (await response.json()) as { token?: unknown }
    if (data && typeof data.token === 'string' && data.token.length > 0) {
      return data.token
    }
  } catch (error) {
    console.error('Error refreshing backend JWT', error)
  }

  return null
}
