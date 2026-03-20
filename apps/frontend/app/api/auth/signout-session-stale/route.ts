import { signOut } from '@/auth'
import {
  getBackendSessionCookie,
  deleteBackendSessionCookie,
} from '@/lib/auth/backend-jwt-cookie.server'
import { getBackendBaseUrlOrThrow } from '@/auth'

export async function GET() {
  // Call backend logout (best-effort, ignore errors)
  try {
    const token = await getBackendSessionCookie()
    if (token) {
      const backendBase = getBackendBaseUrlOrThrow()
      await fetch(`${backendBase}/api/auth/logout`, {
        method: 'POST',
        headers: { Cookie: `backend_session=${token}` },
      })
    }
  } catch {
    // ignore
  }

  await deleteBackendSessionCookie()

  return signOut({ redirectTo: '/' })
}
