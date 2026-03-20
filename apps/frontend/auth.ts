// apps/frontend/auth.ts
import NextAuth from 'next-auth'
import type { NextAuthResult } from 'next-auth'
import Google from 'next-auth/providers/google'

export const BACKEND_BASE_URL_ERROR_MSG =
  'NEXT_PUBLIC_BACKEND_BASE_URL must be set to an absolute URL for backend session authentication'

/**
 * Get and validate backend base URL for server-side use.
 * Prefers BACKEND_BASE_URL (internal) when set; otherwise NEXT_PUBLIC_BACKEND_BASE_URL.
 * Client always uses NEXT_PUBLIC_BACKEND_BASE_URL.
 * Throws if missing or not an absolute http(s) URL.
 * Only throws when called (lazy evaluation).
 */
export function getBackendBaseUrlOrThrow(): string {
  const url =
    process.env.BACKEND_BASE_URL || process.env.NEXT_PUBLIC_BACKEND_BASE_URL
  if (!url) {
    throw new Error(BACKEND_BASE_URL_ERROR_MSG)
  }

  try {
    const parsed = new URL(url)
    if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
      throw new Error(BACKEND_BASE_URL_ERROR_MSG)
    }
    return url
  } catch (error) {
    if (error instanceof TypeError) {
      throw new Error(BACKEND_BASE_URL_ERROR_MSG)
    }
    throw error
  }
}

const nextAuthResult = NextAuth({
  secret: process.env.AUTH_SECRET,
  trustHost: true, // Required when behind a reverse proxy like Caddy

  session: {
    strategy: 'jwt',
    // Shorter session lifetime during early rollout to reduce exposure window
    maxAge: 14 * 24 * 60 * 60, // 14 days
  },
  providers: [
    Google({
      allowDangerousEmailAccountLinking: false,
      // Required for account.id_token in JWT callback; without it Google may not
      // include the ID token, causing backend verification to fail.
      idToken: true,
    } as Parameters<typeof Google>[0]),
  ],
  callbacks: {
    async jwt({ token, account, profile }) {
      if (account?.provider === 'google' && profile) {
        const idToken = await import('@/lib/auth/require-google-id-token').then(
          (m) => m.requireGoogleIdToken(account)
        )

        if (!profile.email) {
          throw new Error('Google profile missing email')
        }
        if (!profile.sub) {
          throw new Error('Google profile missing sub')
        }

        token.email = profile.email
        token.googleSub = profile.sub
        token.name = profile.name || token.name

        const backendBase = getBackendBaseUrlOrThrow()

        try {
          const response = await fetch(`${backendBase}/api/auth/login`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ id_token: idToken }),
          })

          if (response.ok) {
            const data = await response.json()
            if (data && typeof data.token === 'string') {
              const { setBackendSessionCookie } =
                await import('@/lib/auth/backend-jwt-cookie.server')
              await setBackendSessionCookie(data.token)
            }
          }
        } catch (error) {
          const { logError } = await import('@/lib/logging/error-logger')
          logError('Failed to get backend session on initial login', error, {
            action: 'initialLogin',
          })
          throw error
        }
      }

      return token
    },
  },
})

export const handlers: NextAuthResult['handlers'] = nextAuthResult.handlers
export const auth: NextAuthResult['auth'] = nextAuthResult.auth
export const signIn: NextAuthResult['signIn'] = nextAuthResult.signIn
export const signOut: NextAuthResult['signOut'] = nextAuthResult.signOut
