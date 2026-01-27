// Server-only helpers for backend JWT cookie

import { cookies } from 'next/headers'
import { BACKEND_JWT_COOKIE_NAME } from '@/lib/auth/backend-jwt-cookie'

export async function getBackendJwtFromCookie(): Promise<string | null> {
  const cookieStore = await cookies()
  return cookieStore.get(BACKEND_JWT_COOKIE_NAME)?.value ?? null
}

export async function setBackendJwtInCookie(token: string): Promise<void> {
  const cookieStore = await cookies()
  cookieStore.set(BACKEND_JWT_COOKIE_NAME, token, {
    httpOnly: true,
    secure: process.env.NODE_ENV === 'production',
    sameSite: 'lax',
    maxAge: 60 * 60 * 24,
    path: '/',
  })
}

export async function deleteBackendJwtCookie(): Promise<void> {
  const cookieStore = await cookies()

  // Most reliable delete: overwrite with maxAge=0 on '/'
  cookieStore.set(BACKEND_JWT_COOKIE_NAME, '', { path: '/', maxAge: 0 })
}
