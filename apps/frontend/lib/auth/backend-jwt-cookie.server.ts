// Server-only helpers for backend session cookie

import { cookies } from 'next/headers'
import { BACKEND_SESSION_COOKIE_NAME } from '@/lib/auth/backend-jwt-cookie'

export async function getBackendSessionCookie(): Promise<string | null> {
  const cookieStore = await cookies()
  return cookieStore.get(BACKEND_SESSION_COOKIE_NAME)?.value ?? null
}

export async function setBackendSessionCookie(token: string): Promise<void> {
  const cookieStore = await cookies()
  cookieStore.set(BACKEND_SESSION_COOKIE_NAME, token, {
    httpOnly: true,
    secure: process.env.NODE_ENV === 'production',
    sameSite: 'lax',
    maxAge: 60 * 60 * 24,
    path: '/',
  })
}

export async function deleteBackendSessionCookie(): Promise<void> {
  const cookieStore = await cookies()

  // Most reliable delete: overwrite with maxAge=0 on '/'
  cookieStore.set(BACKEND_SESSION_COOKIE_NAME, '', { path: '/', maxAge: 0 })
}
