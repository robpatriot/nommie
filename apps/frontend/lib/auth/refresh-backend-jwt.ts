// Backend JWT refresh logic
// This file provides both pure functions (that never set cookies) and context-specific wrappers

import { auth, getBackendBaseUrlOrThrow } from '@/auth'
import { cookies } from 'next/headers'
import { BACKEND_JWT_COOKIE_NAME } from './backend-jwt-cookie'
import { getBackendJwtFromCookie } from './backend-jwt-cookie.server'
import { checkBackendHealth } from '@/lib/server/backend-health'
import {
  markBackendUp,
  shouldLogError,
  isInStartupWindow,
} from '@/lib/server/backend-status'
import { isBackendConnectionError } from '@/lib/server/connection-errors'
import { parseErrorResponse } from '@/lib/api/error-parsing'
import * as jose from 'jose'
import { logError, logWarning } from '@/lib/logging/error-logger'

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
 * Result of refreshing backend JWT
 */
export interface RefreshBackendJwtResult {
  token: string
  refreshed: boolean
}

/**
 * Pure function that refreshes backend JWT if needed.
 * This function never sets cookies; cookie writes must be done by wrappers
 * in server actions, route handlers, or proxy.
 *
 * @param getCookie - Callback to read the backend JWT cookie
 * @param cookieHeader - Optional cookie header string (for proxy context)
 * @returns Object with token and refreshed flag, or null if refresh failed
 * @throws BackendJwtError if JWT cannot be obtained and no fallback exists
 */
export async function refreshBackendJwtIfNeeded(
  getCookie: (name: string) => Promise<string | undefined>,
  cookieHeader?: string
): Promise<RefreshBackendJwtResult | null> {
  // Get existing JWT from cookie
  const existing = await getCookie(BACKEND_JWT_COOKIE_NAME)

  // Check if existing JWT is valid
  if (
    existing &&
    !isJwtExpired(existing) &&
    !isJwtExpiringSoon(existing, REFRESH_THRESHOLD_SECONDS)
  ) {
    return { token: existing, refreshed: false }
  }

  // Need to refresh - fetch new JWT
  const newToken = await fetchNewBackendJwt(cookieHeader)

  if (!newToken) {
    // Refresh failed
    if (existing && !isJwtExpired(existing)) {
      // Existing JWT is still valid (just expiring soon) - use it
      return { token: existing, refreshed: false }
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

  return { token: newToken, refreshed: true }
}

/**
 * Fetch a new backend JWT from the backend
 * @param cookieHeader - Optional cookie header string (for proxy context)
 */
async function fetchNewBackendJwt(
  cookieHeader?: string
): Promise<string | null> {
  // In proxy context, we can't use auth() or cookies() from next/headers
  // So we need the cookie header passed in
  if (!cookieHeader) {
    // Try to get from next/headers (works in server actions/route handlers)
    try {
      const session = await auth()
      if (!session?.user?.email) {
        return null
      }

      // Get googleSub from token (needed for backend auth)
      const cookieStore = await cookies()
      cookieHeader = cookieStore
        .getAll()
        .map(({ name, value }) => `${name}=${value}`)
        .join('; ')

      if (!cookieHeader) {
        return null
      }
    } catch {
      // If auth() or cookies() fail (e.g., in proxy), return null
      return null
    }
  }

  // Parse cookie header to get session info
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

  // If getToken returns null, the NextAuth session is expired or invalid
  // In this case, we cannot refresh the backend JWT, so return null
  // The caller should clear the backend JWT cookie and allow the request to proceed
  if (!token) {
    return null
  }

  const googleSub =
    (token?.googleSub && typeof token.googleSub === 'string'
      ? token.googleSub
      : null) ||
    (token?.sub && typeof token.sub === 'string' ? token.sub : null)

  if (!googleSub) {
    return null
  }

  // Get email and name from token
  const email =
    token?.email && typeof token.email === 'string' ? token.email : null
  const name =
    token?.name && typeof token.name === 'string' ? token.name : undefined

  if (!email) {
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
      logError(
        'Backend not available for JWT refresh',
        new Error('Backend unavailable')
      )
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
          logWarning('Backend JWT refresh rate limited (429)')
        }
        return null
      }

      if (shouldLogError()) {
        logError(
          'Backend JWT refresh failed',
          new Error(`HTTP ${response.status}`),
          {
            status: response.status,
          }
        )
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
      // Let this propagate to fetchWithAuth()
      // where it will be surfaced as a 403 EMAIL_NOT_ALLOWED.
      throw error
    }

    const isConnectionError = isBackendConnectionError(error)

    if (shouldLogError()) {
      if (isConnectionError) {
        logError('Backend connection error during JWT refresh', error)
      } else {
        logError('Error refreshing backend JWT', error)
      }
    }
    return null
  }
}

/**
 * Get cookie options for backend JWT cookie
 */
function getBackendJwtCookieOptions() {
  return {
    httpOnly: true,
    secure: process.env.NODE_ENV === 'production',
    sameSite: 'lax' as const,
    maxAge: 60 * 60 * 24, // 24 hours
    path: '/',
  }
}

/**
 * Ensure we have a valid backend JWT for server actions and route handlers.
 * Refreshes if needed and sets cookie if refreshed.
 * Use this in server actions and route handlers (where cookies can be modified).
 *
 * @throws BackendJwtError if JWT cannot be obtained
 */
export async function ensureBackendJwtForServerAction(): Promise<string> {
  const cookieStore = await cookies()
  const getCookie = async (name: string) => {
    return cookieStore.get(name)?.value
  }

  // Get cookie header for fetchNewBackendJwt (optional, will use auth() if not provided)
  const cookieHeader = cookieStore
    .getAll()
    .map(({ name, value }) => `${name}=${value}`)
    .join('; ')

  const result = await refreshBackendJwtIfNeeded(getCookie, cookieHeader)

  if (!result) {
    throw new BackendJwtError('Authentication required')
  }

  // If refreshed, set the cookie
  if (result.refreshed) {
    cookieStore.set(
      BACKEND_JWT_COOKIE_NAME,
      result.token,
      getBackendJwtCookieOptions()
    )
  }

  return result.token
}

/**
 * Get backend JWT without refreshing (read-only).
 * Use this in Server Components where cookies cannot be modified.
 *
 * @returns The backend JWT token, or null if not available
 */
export async function getBackendJwtReadOnly(): Promise<string | null> {
  return await getBackendJwtFromCookie()
}
