// Server-only helper to get backend JWT from NextAuth token
// This file must never be imported by client code

import { cookies } from 'next/headers'
import * as jose from 'jose'

/**
 * Get the backend JWT from the server-side NextAuth token.
 * This is server-only and should never be called from client code.
 *
 * @returns The backend JWT string, or undefined if not available
 */
export async function getBackendJwtServer(): Promise<string | undefined> {
  const cookieStore = await cookies()
  // NextAuth v5 uses 'authjs.session-token' or '__Secure-authjs.session-token' (in production)
  const sessionToken =
    cookieStore.get('authjs.session-token')?.value ||
    cookieStore.get('__Secure-authjs.session-token')?.value

  if (!sessionToken) {
    return undefined
  }

  const secret = process.env.AUTH_SECRET
  if (!secret) {
    return undefined
  }

  try {
    const secretKey = new TextEncoder().encode(secret)
    const { payload } = await jose.jwtVerify(sessionToken, secretKey)
    // payload is JWTPayload which has an index signature, so we can safely access backendJwt
    const backendJwt = payload.backendJwt
    return typeof backendJwt === 'string' ? backendJwt : undefined
  } catch {
    return undefined
  }
}
