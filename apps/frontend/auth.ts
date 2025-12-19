// apps/frontend/auth.ts
import NextAuth from 'next-auth'
import type { NextAuthResult } from 'next-auth'
import Google from 'next-auth/providers/google'

export const BACKEND_BASE_URL_ERROR_MSG =
  'BACKEND_BASE_URL must be set to an absolute URL when minting backend JWT'

/**
 * Get and validate BACKEND_BASE_URL.
 * Throws if missing or not an absolute http(s) URL.
 * Only throws when called (lazy evaluation).
 */
export function getBackendBaseUrlOrThrow(): string {
  const url = process.env.BACKEND_BASE_URL
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
    }),
  ],
  callbacks: {
    async jwt({ token, account, profile, trigger }) {
      // Store user info in token for refreshing backend JWT
      if (account?.provider === 'google' && profile) {
        if (!profile.email) {
          throw new Error('Google profile missing email')
        }
        if (!profile.sub) {
          throw new Error('Google profile missing sub')
        }
        token.email = profile.email
        token.googleSub = profile.sub
        token.name = profile.name || token.name

        // Store backend JWT in cookie on initial login
        // (Also stored in token for backward compatibility during migration)
        const backendBase = getBackendBaseUrlOrThrow()

        try {
          const response = await fetch(`${backendBase}/api/auth/login`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              email: profile.email,
              name: profile.name,
              google_sub: profile.sub,
            }),
          })

          if (response.ok) {
            const data = await response.json()
            if (data && typeof data.token === 'string') {
              // Store in token for backward compatibility
              token.backendJwt = data.token

              // Also store in cookie (new approach)
              const { setBackendJwtInCookie } =
                await import('@/lib/auth/backend-jwt-cookie')
              await setBackendJwtInCookie(data.token)
            }
          }
        } catch (error) {
          const { logError } = await import('@/lib/logging/error-logger')
          logError('Failed to get backend JWT on initial login', error, {
            action: 'initialLogin',
          })
          // Don't fail the login if backend JWT fetch fails
        }
      }

      // Handle updates triggered by unstable_update() from server-side code
      // This allows server-side refresh to persist the new JWT to the token
      if (trigger === 'update' && token.backendJwt) {
        // Token already updated by unstable_update(), just return it
        return token
      }

      return token
    },
  },
})

export const handlers: NextAuthResult['handlers'] = nextAuthResult.handlers
export const auth: NextAuthResult['auth'] = nextAuthResult.auth
export const signIn: NextAuthResult['signIn'] = nextAuthResult.signIn
export const signOut: NextAuthResult['signOut'] = nextAuthResult.signOut
export const unstable_update: NextAuthResult['unstable_update'] =
  nextAuthResult.unstable_update
