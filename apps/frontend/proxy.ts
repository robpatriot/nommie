import { NextResponse } from 'next/server'
import type { NextRequest } from 'next/server'
import { ensureBackendJwtForMiddleware } from '@/lib/auth/refresh-backend-jwt'

/**
 * Proxy to ensure backend JWT is valid and refreshed if needed.
 * Runs on routes that require backend authentication.
 *
 * This proxy:
 * - Checks if backend JWT exists and is valid
 * - Refreshes it if needed (expired or expiring soon)
 * - Sets the cookie on the response if refreshed
 * - Does not block requests - errors are handled gracefully
 */
export async function proxy(request: NextRequest) {
  // Only run on routes that need backend auth
  const pathname = request.nextUrl.pathname

  // Skip proxy for:
  // - Auth routes (/api/auth/*)
  // - Static files
  // - Public routes that don't need backend auth
  if (
    pathname.startsWith('/api/auth/') ||
    pathname.startsWith('/_next/') ||
    pathname.startsWith('/favicon') ||
    pathname === '/'
  ) {
    return NextResponse.next()
  }

  // For routes that need backend auth, ensure JWT is valid
  const response = NextResponse.next()

  try {
    await ensureBackendJwtForMiddleware(request, response)
  } catch (error) {
    // Log error but don't block the request
    // The page/route handler will handle auth errors appropriately
    console.warn('Proxy JWT refresh error:', error)
  }

  return response
}

export const config = {
  matcher: [
    /*
     * Match all request paths except for the ones starting with:
     * - api/auth (NextAuth routes)
     * - _next/static (static files)
     * - _next/image (image optimization files)
     * - favicon.ico (favicon file)
     */
    '/((?!api/auth|_next/static|_next/image|favicon.ico).*)',
  ],
}
