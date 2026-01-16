// Middleware-safe JWT refresh logic
// This file MUST NOT import '@/auth' or 'next-auth' (except for types/jwt) to stay Edge-compatible

import { BACKEND_JWT_COOKIE_NAME } from './backend-jwt-cookie'
import * as jose from 'jose'
import { getToken } from 'next-auth/jwt'

const REFRESH_THRESHOLD_SECONDS = 300 // 5 minutes

export class BackendJwtError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'BackendJwtError'
  }
}

// Re-implement pure functions here to avoid importing from files that import 'server-only' or heavy libs

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
 * Check if a JWT is expiring soon
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

export interface RefreshBackendJwtResult {
  token: string
  refreshed: boolean
}

/**
 * Get cookie options for backend JWT cookie
 */
export function getBackendJwtCookieOptions() {
  return {
    httpOnly: true,
    secure: process.env.NODE_ENV === 'production',
    sameSite: 'lax' as const,
    maxAge: 60 * 60 * 24, // 24 hours
    path: '/',
  }
}

/**
 * Pure function to refresh backend JWT.
 * Compatible with Middleware (Edge).
 */
export async function refreshBackendJwtIfNeeded(
  getCookie: (name: string) => Promise<string | undefined>,
  cookieHeader?: string
): Promise<RefreshBackendJwtResult | null> {
  // Get existing JWT from cookie
  const existing = await getCookie(BACKEND_JWT_COOKIE_NAME)

  // Check valid
  if (
    existing &&
    !isJwtExpired(existing) &&
    !isJwtExpiringSoon(existing, REFRESH_THRESHOLD_SECONDS)
  ) {
    return { token: existing, refreshed: false }
  }

  // Need refresh
  if (!cookieHeader) {
    // In middleware, we MUST have the cookie header to pass to next-auth
    return null
  }

  const newToken = await fetchNewBackendJwtGeneric(cookieHeader)

  if (!newToken) {
    if (existing && !isJwtExpired(existing)) {
      return { token: existing, refreshed: false }
    }
    // Failed and existing is expired/missing
    // We do NOT throw here for middleware - we just return null so middleware lets the request pass
    // (API handle will catch 401 later)
    return null
  }

  return { token: newToken, refreshed: true }
}

/**
 * Fetch new JWT - Generic version for middleware
 */
async function fetchNewBackendJwtGeneric(
  cookieHeader: string
): Promise<string | null> {
  const headers = new Headers({ cookie: cookieHeader })
  // We need to mock the Request object for getToken
  // getToken expects `req` to have `headers` or `cookies`

  // NOTE: getToken is Edge compatible.
  const token = await getToken({
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    req: { headers } as any,
    secret: process.env.AUTH_SECRET,
    secureCookie: process.env.NODE_ENV === 'production',
    salt:
      process.env.NODE_ENV === 'production'
        ? '__Secure-authjs.session-token'
        : 'authjs.session-token',
  })

  if (!token) return null

  // Extract needed fields
  const googleSub =
    (token?.googleSub && typeof token.googleSub === 'string'
      ? token.googleSub
      : null) ||
    (token?.sub && typeof token.sub === 'string' ? token.sub : null)

  const email = token?.email
  const name = token?.name

  if (!googleSub || !email) return null

  // Get Backend URL - needs env var
  const backendBase = process.env.NEXT_PUBLIC_BACKEND_BASE_URL
  if (!backendBase) return null

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

    if (!response.ok) return null

    const data = (await response.json()) as { token?: unknown }
    if (data && typeof data.token === 'string' && data.token.length > 0) {
      return data.token
    }
  } catch (err) {
    console.error('Middleware: Failed to fetch backend JWT', err)
  }
  return null
}
