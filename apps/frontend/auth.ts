// apps/frontend/auth.ts
import NextAuth from 'next-auth'
import Google from 'next-auth/providers/google'

export const { handlers, auth, signIn, signOut } = NextAuth({
  // NextAuth v5 will auto-infer secret from AUTH_SECRET if not provided.
  // We set it explicitly to support both AUTH_SECRET and APP_JWT_SECRET.
  // NextAuth checks this at init time, so we need to read it here.
  // Note: Next.js loads .env.local before evaluating modules, so this should work.
  secret: process.env.AUTH_SECRET ?? process.env.APP_JWT_SECRET,

  session: { strategy: 'jwt' },
  providers: [
    Google({
      allowDangerousEmailAccountLinking: false,
    }),
  ],
  callbacks: {
    async jwt({ token, account, profile }) {
      // Validate required env vars here (lazy evaluation) after Next.js has loaded env vars
      const authSecret = process.env.AUTH_SECRET ?? process.env.APP_JWT_SECRET
      if (!authSecret) {
        throw new Error('Missing AUTH_SECRET or APP_JWT_SECRET')
      }

      if (account?.provider === 'google' && profile) {
        // Check BACKEND_BASE_URL here (lazy evaluation) after Next.js has loaded env vars
        const backendBase = process.env.BACKEND_BASE_URL
        if (!backendBase) {
          throw new Error('BACKEND_BASE_URL must be set')
        }

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
            const { token: backendJwt } = await response.json()
            token.backendJwt = backendJwt
          } else {
            throw new Error(
              `Backend login failed: ${response.status} ${response.statusText}`
            )
          }
        } catch (error) {
          console.error('Failed to get backend JWT:', error)
          throw error
        }
      }
      return token
    },
    async session({ session, token }) {
      if (token.backendJwt) {
        session.backendJwt = String(token.backendJwt)
      }
      return session
    },
  },
})
