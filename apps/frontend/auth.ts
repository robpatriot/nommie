// apps/frontend/auth.ts
import NextAuth from 'next-auth'
import Google from 'next-auth/providers/google'

const backendBase = process.env.BACKEND_BASE_URL
if (!backendBase) throw new Error('BACKEND_BASE_URL must be set')

// 👇 add this
const authSecret = process.env.AUTH_SECRET ?? process.env.APP_JWT_SECRET
if (!authSecret) {
  throw new Error('Missing AUTH_SECRET or APP_JWT_SECRET')
}

export const { handlers, auth, signIn, signOut } = NextAuth({
  // 👇 wire it into NextAuth
  secret: authSecret,

  session: { strategy: 'jwt' },
  providers: [
    Google({
      allowDangerousEmailAccountLinking: false,
    }),
  ],
  callbacks: {
    async jwt({ token, account, profile }) {
      if (account?.provider === 'google' && profile) {
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
        ;(session as any).backendJwt = token.backendJwt as string
      }
      return session
    },
  },
})

