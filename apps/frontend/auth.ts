// apps/frontend/auth.ts
import NextAuth from 'next-auth'
import type { NextAuthResult } from 'next-auth'
import Google from 'next-auth/providers/google'

export const BACKEND_BASE_URL_ERROR_MSG =
  'NEXT_PUBLIC_BACKEND_BASE_URL must be set to an absolute URL when minting backend JWT'

/**
 * Get and validate NEXT_PUBLIC_BACKEND_BASE_URL.
 * Throws if missing or not an absolute http(s) URL.
 * Only throws when called (lazy evaluation).
 */
export function getBackendBaseUrlOrThrow(): string {
  const url = process.env.NEXT_PUBLIC_BACKEND_BASE_URL
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
    async signIn({ account, profile }) {
      // Check allowlist BEFORE creating session to prevent unnecessary
      // API calls and session creation for non-allowed emails
      if (account?.provider === 'google' && profile?.email) {
        const backendBase = getBackendBaseUrlOrThrow()

        try {
          const checkResponse = await fetch(
            `${backendBase}/api/auth/check-allowlist`,
            {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({ email: profile.email }),
            }
          )

          if (!checkResponse.ok) {
            // Email not allowed - redirect to home with accessDenied parameter
            // This prevents NextAuth from treating it as an error
            if (checkResponse.status === 403) {
              return '/?accessDenied=true'
            }

            // Other errors: log and redirect
            const { logError } = await import('@/lib/logging/error-logger')
            logError(
              'Failed to check allowlist',
              new Error(`HTTP ${checkResponse.status}`),
              { action: 'checkAllowlist' }
            )
            return '/?accessDenied=true'
          }
        } catch (error) {
          // Network or other errors: log and redirect
          const { logError } = await import('@/lib/logging/error-logger')
          logError('Failed to check allowlist', error, {
            action: 'checkAllowlist',
          })
          return '/?accessDenied=true'
        }
      }

      // Allow sign-in to proceed
      return true
    },

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

        // Email is allowed (signIn callback already checked), proceed with backend login
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
              // Store in token and cookie
              token.backendJwt = data.token

              // IMPORTANT: cookie helpers are server-only now
              const { setBackendJwtInCookie } =
                await import('@/lib/auth/backend-jwt-cookie.server')
              await setBackendJwtInCookie(data.token)
            }
          }
        } catch (error) {
          const { logError } = await import('@/lib/logging/error-logger')
          logError('Failed to get backend JWT on initial login', error, {
            action: 'initialLogin',
          })
          // Don't fail the login if backend JWT fetch fails (allowlist already passed)
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
