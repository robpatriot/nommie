import { signOut } from '@/auth'

/**
 * Route handler that signs out the user when their session is stale
 * (backend JWT is invalid but NextAuth session still exists).
 * Signs out and redirects to home page.
 */
export async function GET() {
  return signOut({
    redirectTo: '/',
  })
}
