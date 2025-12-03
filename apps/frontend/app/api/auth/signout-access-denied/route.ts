import { signOut } from '@/auth'

/**
 * Route handler that signs out the user and redirects to the home page
 * with an access denied message. Redirect behavior is delegated entirely
 * to Auth.js via the `signOut` helper.
 */
export async function GET() {
  return signOut({
    redirectTo: '/?accessDenied=true',
  })
}
