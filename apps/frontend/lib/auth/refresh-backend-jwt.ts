// Backend JWT refresh logic
// Works from both Server Components and Server Actions

import { auth, getBackendBaseUrlOrThrow } from '@/auth'
import { cookies } from 'next/headers'
import {
  getBackendJwtFromCookie,
  setBackendJwtInCookie,
} from './backend-jwt-cookie'
import { checkBackendHealth } from '@/lib/server/backend-health'
import {
  markBackendUp,
  shouldLogError,
  isInStartupWindow,
} from '@/lib/server/backend-status'
import { parseErrorResponse } from '@/lib/api/error-parsing'
import * as jose from 'jose'

const REFRESH_THRESHOLD_SECONDS = 300 // 5 minutes - refresh if expiring within this

export class BackendJwtError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'BackendJwtError'
  }
}

/**
 * Check if a JWT is expired
 */
function isJwtExpired(token: string): boolean {
  try {
    const decoded = jose.decodeJwt(token)
    const exp = decoded.exp

    if (typeof exp !== 'number' || !exp) {
      return true
    }

    const nowSeconds = Math.floor(Date.now() / 1000)
    return exp <= nowSeconds
  } catch {
    return true
  }
}

/**
 * Check if a JWT is expiring soon (within threshold)
 */
function isJwtExpiringSoon(token: string, thresholdSeconds: number): boolean {
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

/**
 * Fetch a new backend JWT from the backend
 */
async function fetchNewBackendJwt(): Promise<string | null> {
  const session = await auth()

  if (!session?.user?.email) {
    return null
  }

  // Get user info from session
  const email = session.user.email
  const name = session.user.name || undefined

  // Get googleSub from token (needed for backend auth)
  const cookieStore = await cookies()
  const cookieHeader = cookieStore
    .getAll()
    .map(({ name, value }) => `${name}=${value}`)
    .join('; ')

  if (!cookieHeader) {
    return null
  }

  const headers = new Headers({ cookie: cookieHeader })
  const req: { headers: Headers } = { headers }

  const { getToken } = await import('next-auth/jwt')
  const token = await getToken({
    req,
    secret: process.env.AUTH_SECRET,
    secureCookie: process.env.NODE_ENV === 'production',
    salt:
      process.env.NODE_ENV === 'production'
        ? '__Secure-authjs.session-token'
        : 'authjs.session-token',
  })

  const googleSub =
    (token?.googleSub && typeof token.googleSub === 'string'
      ? token.googleSub
      : null) ||
    (token?.sub && typeof token.sub === 'string' ? token.sub : null)

  if (!googleSub) {
    return null
  }

  // Check backend health first
  const backendHealthy = await checkBackendHealth()
  if (!backendHealthy) {
    if (isInStartupWindow()) {
      // Backend not ready yet - return null silently
      return null
    }
    // Backend should be up by now - log error
    if (shouldLogError()) {
      console.warn('Backend not available for JWT refresh')
    }
    return null
  }

  // Fetch new JWT from backend
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
      markBackendUp() // Got response, backend is up

      // Special-case allowlist failures so callers can distinguish them.
      if (response.status === 403) {
        try {
          const parsed = await parseErrorResponse(response)
          if (parsed.code === 'EMAIL_NOT_ALLOWED') {
            // Propagate a specific signal for allowlist rejection
            throw new BackendJwtError('EMAIL_NOT_ALLOWED')
          }
        } catch (error) {
          // Re-throw if it's our EMAIL_NOT_ALLOWED signal
          if (
            error instanceof BackendJwtError &&
            error.message === 'EMAIL_NOT_ALLOWED'
          ) {
            throw error
          }
          // Fall through to generic handling if parsing fails
        }
      }

      if (response.status === 429) {
        // Rate limited - log but don't throw
        if (shouldLogError()) {
          console.warn('Backend JWT refresh rate limited (429)')
        }
        return null
      }

      if (shouldLogError()) {
        console.warn('Backend JWT refresh failed', response.status)
      }
      return null
    }

    markBackendUp()

    const data = (await response.json()) as { token?: unknown }
    if (data && typeof data.token === 'string' && data.token.length > 0) {
      return data.token
    }

    return null
  } catch (error) {
    // Preserve explicit allowlist failures so callers can distinguish them.
    if (
      error instanceof BackendJwtError &&
      error.message === 'EMAIL_NOT_ALLOWED'
    ) {
      // Let this propagate to ensureBackendJwt() / fetchWithAuth()
      // where it will be surfaced as a 403 EMAIL_NOT_ALLOWED.
      throw error
    }

    const errorMessage =
      error instanceof Error ? error.message.toLowerCase() : ''
    const causeMessage =
      error instanceof Error && 'cause' in error && error.cause instanceof Error
        ? error.cause.message.toLowerCase()
        : ''

    const isConnectionError =
      error instanceof Error &&
      (errorMessage.includes('econnrefused') ||
        errorMessage.includes('fetch failed') ||
        errorMessage.includes('connection') ||
        causeMessage.includes('econnrefused') ||
        causeMessage.includes('connect econnrefused'))

    if (shouldLogError()) {
      if (isConnectionError) {
        console.warn('Backend connection error during JWT refresh', error)
      } else {
        console.warn('Error refreshing backend JWT', error)
      }
    }
    return null
  }
}

/**
 * Ensure we have a valid backend JWT.
 * Refreshes if needed. Works from both Server Components and Server Actions.
 *
 * @throws BackendJwtError if JWT cannot be obtained
 */
export async function ensureBackendJwt(): Promise<string> {
  // Get existing JWT from cookie
  const existing = await getBackendJwtFromCookie()

  // Check if existing JWT is valid
  if (
    existing &&
    !isJwtExpired(existing) &&
    !isJwtExpiringSoon(existing, REFRESH_THRESHOLD_SECONDS)
  ) {
    return existing
  }

  // Need to refresh - fetch new JWT
  const newToken = await fetchNewBackendJwt()

  if (!newToken) {
    // Refresh failed
    if (existing && !isJwtExpired(existing)) {
      // Existing JWT is still valid (just expiring soon) - use it
      return existing
    }

    // No valid JWT available
    if (isInStartupWindow()) {
      const backendHealthy = await checkBackendHealth()
      if (!backendHealthy) {
        throw new BackendJwtError(
          'Backend is starting up, please try again shortly'
        )
      }
    }
    throw new BackendJwtError('Authentication required')
  }

  // Save new JWT to cookie
  await setBackendJwtInCookie(newToken)
  return newToken
}
