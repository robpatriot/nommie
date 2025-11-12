// apps/frontend/auth.ts
import NextAuth from 'next-auth'
import Google from 'next-auth/providers/google'
import * as jose from 'jose'

/**
 * Check if backend JWT needs to be refreshed.
 * Returns true if JWT is missing or will expire within 5 minutes.
 */
function shouldRefreshBackendJwt(jwt?: string): boolean {
  if (!jwt) {
    return true
  }

  try {
    const decoded = jose.decodeJwt(jwt)
    const exp = decoded.exp

    // Type guard: exp must be a number
    if (typeof exp !== 'number' || !exp) {
      return true
    }

    // Check if expires within 5 minutes (5 * 60 * 1000 ms)
    const expiresIn = exp * 1000 - Date.now()
    return expiresIn < 5 * 60 * 1000
  } catch {
    // Parse errors mean we should refresh
    return true
  }
}

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

export const { handlers, auth, signIn, signOut, unstable_update } = NextAuth({
  // NextAuth v5 will auto-infer secret from AUTH_SECRET if not provided.
  // We set it explicitly to use AUTH_SECRET only on the frontend.
  // Note: Next.js loads .env.local before evaluating modules, so this should work.
  secret: process.env.AUTH_SECRET,

  session: {
    strategy: 'jwt',
    maxAge: 30 * 24 * 60 * 60, // 30 days
  },
  providers: [
    Google({
      allowDangerousEmailAccountLinking: false,
    }),
  ],
  callbacks: {
    async jwt({ token, account, profile, trigger }) {
      // Refresh backend JWT on 'update' trigger as well
      const shouldRefreshOnTrigger = trigger === 'update'
      // Validate required env vars here (lazy evaluation) after Next.js has loaded env vars
      const authSecret = process.env.AUTH_SECRET
      if (!authSecret) {
        throw new Error('Missing AUTH_SECRET')
      }

      // Store user info in token for refreshing backend JWT
      // Require proper data from the initial login
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
      }

      // Proactive backend JWT refresh: refresh if missing or within 5 minutes of expiry
      // Also refresh on 'update' trigger
      const currentJwt =
        typeof token.backendJwt === 'string' ? token.backendJwt : undefined
      const needsRefresh =
        (shouldRefreshOnTrigger || shouldRefreshBackendJwt(currentJwt)) &&
        token.email &&
        token.googleSub

      if (needsRefresh) {
        // Validate BACKEND_BASE_URL (throws if invalid)
        const backendBase = getBackendBaseUrlOrThrow()

        try {
          const response = await fetch(`${backendBase}/api/auth/login`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              email: token.email,
              name: token.name,
              google_sub: token.googleSub,
            }),
          })

          if (response.ok) {
            const data = await response.json()
            // Validate response: ensure token is a string
            if (data && typeof data.token === 'string') {
              token.backendJwt = data.token
            }
          }
          // On non-200, leave token.backendJwt undefined and continue
          // The app should handle 401s gracefully elsewhere
        } catch (error) {
          // On network errors, leave token.backendJwt undefined and continue
          console.error('Failed to refresh backend JWT:', error)
        }
      }
      return token
    },
    async session({ session }) {
      // backendJwt is NOT attached to session - it remains server-only in the JWT token
      return session
    },
  },
})
