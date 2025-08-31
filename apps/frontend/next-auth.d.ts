import NextAuth from 'next-auth'

declare module 'next-auth' {
  interface Session {
    backendJwt?: string
  }
}

declare module 'next-auth/jwt' {
  interface JWT {
    backendJwt?: string
  }
}
