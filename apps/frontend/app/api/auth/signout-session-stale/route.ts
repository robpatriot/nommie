import { signOut } from '@/auth'
import { deleteBackendJwtCookie } from '@/lib/auth/backend-jwt-cookie.server'

export async function GET() {
  // Clear backend JWT cookie server-side
  await deleteBackendJwtCookie()

  return signOut({ redirectTo: '/' })
}
