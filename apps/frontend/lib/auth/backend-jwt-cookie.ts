// Cookie-based storage for backend JWT
// This works from both Server Components and Server Actions

import { cookies } from 'next/headers'

const BACKEND_JWT_COOKIE_NAME = 'backend_jwt'

/**
 * Get the backend JWT from cookie.
 * Returns null if not found or expired.
 */
export async function getBackendJwtFromCookie(): Promise<string | null> {
  const cookieStore = await cookies()
  return cookieStore.get(BACKEND_JWT_COOKIE_NAME)?.value ?? null
}

/**
 * Set the backend JWT in cookie.
 * Works from both Server Components and Server Actions.
 */
export async function setBackendJwtInCookie(token: string): Promise<void> {
  const cookieStore = await cookies()
  cookieStore.set(BACKEND_JWT_COOKIE_NAME, token, {
    httpOnly: true,
    secure: process.env.NODE_ENV === 'production',
    sameSite: 'lax',
    maxAge: 60 * 60 * 24, // 24 hours (backend tokens expire in 5 minutes, but we refresh proactively)
    path: '/',
  })
}

/**
 * Delete the backend JWT cookie.
 */
export async function deleteBackendJwtCookie(): Promise<void> {
  const cookieStore = await cookies()
  cookieStore.delete(BACKEND_JWT_COOKIE_NAME)
}
