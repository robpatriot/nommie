import { NextResponse } from 'next/server'
import type { NextRequest } from 'next/server'
import {
  refreshBackendJwtIfNeeded,
  getBackendJwtCookieOptions,
} from '@/lib/auth/middleware-refresh-utils'
import { BACKEND_JWT_COOKIE_NAME } from '@/lib/auth/backend-jwt-cookie'

export async function proxy(request: NextRequest) {
  // Only run on specific paths if needed, or exclude static files
  if (
    request.nextUrl.pathname.startsWith('/_next') ||
    request.nextUrl.pathname.startsWith('/static') ||
    request.nextUrl.pathname.startsWith('/favicon.ico') ||
    request.nextUrl.pathname.match(/\.(png|jpg|jpeg|gif|svg)$/)
  ) {
    return NextResponse.next()
  }

  const response = NextResponse.next()

  // Helper to extract cookie from request
  const getCookie = async (name: string) => {
    return request.cookies.get(name)?.value
  }

  // Get the entire cookie header string for forwarding to the backend (needed for NextAuth validation)
  const cookieHeader = request.headers.get('cookie') || ''

  try {
    // Attempt to refresh the JWT if needed
    // This pure function does not set cookies itself, it returns the result
    const result = await refreshBackendJwtIfNeeded(getCookie, cookieHeader)

    // If a refresh occurred (new token returned), update the response cookie
    if (result && result.refreshed) {
      const options = getBackendJwtCookieOptions()
      response.cookies.set({
        name: BACKEND_JWT_COOKIE_NAME,
        value: result.token,
        ...options,
      })
    }
  } catch (error) {
    // If refresh fails (e.g. backend down, invalid session), we don't block the request here.
    // We let it proceed. The component/API call attempting to USE the token will fail
    // (with 401), which will trigger the redirect logic in `api.ts`.
    // This avoids middleware acting as a hard gate that could cause redirect loops if not careful.
    // Unique exception: If we KNOW the session is dead, we could delete the cookie?
    // For now, simple is better: fail open in middleware, fail closed in API/Components.
    console.error('Middleware JWT refresh failed:', error)
  }

  return response
}

export const config = {
  matcher: [
    /*
     * Match all request paths except for the ones starting with:
     * - api (API routes)
     * - _next/static (static files)
     * - _next/image (image optimization files)
     * - favicon.ico (favicon file)
     */
    '/((?!api|_next/static|_next/image|favicon.ico).*)',
  ],
}
