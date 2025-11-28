// Server-only helper to get backend JWT from NextAuth token
// This file must never be imported by client code

import { auth, getBackendBaseUrlOrThrow } from '@/auth'
import { getToken } from 'next-auth/jwt'
import { cookies } from 'next/headers'
import { redirect } from 'next/navigation'
import type { Session } from 'next-auth'
import * as jose from 'jose'

interface NextAuthToken {
  backendJwt?: unknown
  email?: unknown
  name?: unknown
  googleSub?: unknown
  sub?: unknown
}

const REFRESH_THRESHOLD_SECONDS = 300 // 5 minutes - consistent with backend token expiry

// Request-level deduplication: track in-flight refresh requests
const refreshPromises = new Map<string, Promise<string | null>>()

export type BackendJwtResolution =
  | { state: 'missing-session'; session: null }
  | { state: 'missing-jwt'; session: Session }
  | { state: 'ready'; session: Session; backendJwt: string }

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
      secureCookie: process.env.NODE_ENV === 'production',
      // Auth.js v5: salt should match the session cookie name
      salt:
        process.env.NODE_ENV === 'production'
          ? '__Secure-authjs.session-token'
          : 'authjs.session-token',
      // Note: getToken() should auto-detect from Next.js context, but manual req breaks this
    })) as NextAuthToken | null

    const existingJwt =
      token?.backendJwt && typeof token.backendJwt === 'string'
        ? token.backendJwt
        : undefined

    // Check if JWT exists and is still valid (not expiring within threshold)
    if (existingJwt && !isJwtExpiring(existingJwt, REFRESH_THRESHOLD_SECONDS)) {
      return { state: 'ready', session, backendJwt: existingJwt }
    }

    // Create deduplication key based on user identity
    // This ensures parallel requests from the same user share the same refresh request
    const dedupeKey =
      (token?.email && typeof token.email === 'string' ? token.email : '') +
      '|' +
      (token?.googleSub && typeof token.googleSub === 'string'
        ? token.googleSub
        : '')

    // Refresh backend JWT if needed
    const refreshed = await refreshBackendJwt({
      session,
      token,
      dedupeKey,
    })
    if (refreshed) {
      return { state: 'ready', session, backendJwt: refreshed }
    }
  } catch (error) {
    console.error('Failed to resolve backend JWT', error)
  }

  return { state: 'missing-jwt', session }
}

export async function requireBackendJwt(): Promise<string> {
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
    const decoded = jose.decodeJwt(token)
    const exp = decoded.exp

    if (typeof exp !== 'number' || !exp) {
      return true
    }

    const nowSeconds = Math.floor(Date.now() / 1000)
    return exp - nowSeconds <= thresholdSeconds
  } catch {
    return true
  }
}

async function refreshBackendJwt({
  session,
  token,
  dedupeKey,
}: {
  session: Session
  token: NextAuthToken | null
  dedupeKey: string
}): Promise<string | null> {
  // Check if refresh is already in flight
  const existingPromise = refreshPromises.get(dedupeKey)
  if (existingPromise) {
    return existingPromise
  }

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

  // Create refresh promise
  const refreshPromise = (async () => {
    try {
      const backendBase = getBackendBaseUrlOrThrow()

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
        // Note: We cannot use unstable_update() here because it modifies cookies,
        // which can only be done in Server Actions or Route Handlers.
        // The refreshed JWT will be returned and used for the current request.
        // On the next request, if the JWT is still missing/expired, it will be refreshed again.
        // This is acceptable since the deduplication prevents duplicate refresh requests.
        return data.token
      }

      return null
    } catch (error) {
      // Check if this is a connection error (backend not ready yet)
      const errorMessage =
        error instanceof Error ? error.message.toLowerCase() : ''
      // Access cause property safely (may not exist in all TypeScript lib versions)
      const causeMessage =
        error instanceof Error &&
        'cause' in error &&
        error.cause instanceof Error
          ? error.cause.message.toLowerCase()
          : ''

      const isConnectionError =
        error instanceof Error &&
        (errorMessage.includes('econnrefused') ||
          errorMessage.includes('fetch failed') ||
          errorMessage.includes('connection') ||
          causeMessage.includes('econnrefused') ||
          causeMessage.includes('connect econnrefused'))

      if (isConnectionError) {
        // Backend not ready yet - this is expected during startup, log at debug level
        // The page will work once the backend starts, or on the next request
        if (process.env.NODE_ENV === 'development') {
          console.debug(
            'Backend not ready yet, JWT refresh will retry on next request'
          )
        }
      } else {
        // Other errors should still be logged
        console.warn('Error refreshing backend JWT', error)
      }
      return null
    } finally {
      // Clean up deduplication cache immediately after completion
      // The delay is handled by checking if promise exists before creating new one
      refreshPromises.delete(dedupeKey)
    }
  })()

  // Store promise for deduplication
  refreshPromises.set(dedupeKey, refreshPromise)

  return refreshPromise
}
