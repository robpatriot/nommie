// apps/frontend/auth.ts
import NextAuth from 'next-auth'
import Google from 'next-auth/providers/google'

export const { handlers, auth, signIn, signOut } = NextAuth({
  session: { strategy: 'jwt' },
  providers: [
    Google({
      allowDangerousEmailAccountLinking: false,
    }),
  ],
  callbacks: {
    async jwt({ token, account, profile }) {
      // Only run this on initial sign-in
      if (account?.provider === 'google' && profile) {
        try {
          const response = await fetch(
            `${process.env.NEXT_PUBLIC_BACKEND_BASE_URL}/api/auth/login`,
            {
              method: 'POST',
              headers: {
                'Content-Type': 'application/json',
              },
              body: JSON.stringify({
                email: profile.email,
                name: profile.name,
                google_sub: profile.sub,
              }),
            }
          )

          if (response.ok) {
            const { token: backendJwt } = await response.json()
            token.backendJwt = backendJwt
          } else {
            // Fail fast on backend login failure to avoid session without backendJwt
            throw new Error(
              `Backend login failed: ${response.status} ${response.statusText}`
            )
          }
        } catch (error) {
          console.error('Failed to get backend JWT:', error)
          // Re-throw to cancel the sign-in process
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

// Route handlers are re-exported in apps/frontend/app/api/auth/[...nextauth]/route.ts
