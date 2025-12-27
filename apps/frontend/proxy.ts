import { NextResponse } from 'next/server'
import type { NextRequest } from 'next/server'
import { ensureBackendJwtForMiddleware } from '@/lib/auth/refresh-backend-jwt'
import { BACKEND_JWT_COOKIE_NAME } from '@/lib/auth/backend-jwt-cookie'
import { LOCALE_COOKIE_NAME, resolveLocale } from '@/i18n/locale'

/**
 * Proxy to ensure backend JWT is valid and refreshed if needed, and handle locale resolution.
 * Runs on routes that require backend authentication.
 *
 * This proxy:
 * - Resolves locale from cookie or Accept-Language header
 * - Sets locale cookie if needed
 * - Sets x-next-intl-locale header for next-intl
 * - Checks if backend JWT exists and is valid
 * - Refreshes JWT if needed (expired or expiring soon)
 * - Sets the cookie on the response if refreshed
 * - Does not block requests - errors are handled gracefully
 */
export async function proxy(request: NextRequest) {
  const pathname = request.nextUrl.pathname

  // Resolve locale from cookie or Accept-Language header
  const { locale } = resolveLocale({
    cookieLocale: request.cookies.get(LOCALE_COOKIE_NAME)?.value ?? null,
    acceptLanguageHeader: request.headers.get('accept-language'),
  })

  const requestHeaders = new Headers(request.headers)
  requestHeaders.set('x-next-intl-locale', locale)

  // Create response with locale header
  const response = NextResponse.next({
    request: {
      headers: requestHeaders,
    },
  })

  // Set locale cookie if it doesn't match resolved locale
  const existingLocaleCookie = request.cookies.get(LOCALE_COOKIE_NAME)?.value
  if (existingLocaleCookie !== locale) {
    response.cookies.set(LOCALE_COOKIE_NAME, locale, {
      httpOnly: false,
      sameSite: 'lax',
      secure: process.env.NODE_ENV === 'production',
      path: '/',
      maxAge: 60 * 60 * 24 * 365,
    })
  }

  // Skip JWT refresh for:
  // - Auth routes (/api/auth/*)
  // - Static files
  // But we still need to check and clear expired cookies even for public routes like '/'
  const shouldCheckJwt = !(
    pathname.startsWith('/api/auth/') ||
    pathname.startsWith('/_next/') ||
    pathname.startsWith('/favicon')
  )

  // For routes that need backend auth, ensure JWT is valid
  // Also check for expired NextAuth session on all routes (including '/') to clear stale cookies
  if (shouldCheckJwt) {
    try {
      const jwt = await ensureBackendJwtForMiddleware(request, response)

      // If JWT refresh failed, check if NextAuth session is expired
      // Only clear the backend JWT cookie if NextAuth session is expired
      // (don't clear on transient errors like backend downtime)
      if (!jwt) {
        // Check if NextAuth session is expired by trying to get the token
        const { getToken } = await import('next-auth/jwt')
        const headers = new Headers({
          cookie: request.headers.get('cookie') || '',
        })
        const req: { headers: Headers } = { headers }

        const token = await getToken({
          req,
          secret: process.env.AUTH_SECRET,
          secureCookie: process.env.NODE_ENV === 'production',
          salt:
            process.env.NODE_ENV === 'production'
              ? '__Secure-authjs.session-token'
              : 'authjs.session-token',
        })

        // If NextAuth token is null, the session is expired - clear backend JWT cookie
        // This allows the user to access the home page to log in again
        if (!token) {
          response.cookies.delete(BACKEND_JWT_COOKIE_NAME)
        }
      }
    } catch (error) {
      // Log error but don't block the request
      // The page/route handler will handle auth errors appropriately
      console.warn('[proxy] JWT refresh error:', error)

      // Check if NextAuth session is expired before clearing cookie
      try {
        const { getToken } = await import('next-auth/jwt')
        const headers = new Headers({
          cookie: request.headers.get('cookie') || '',
        })
        const req: { headers: Headers } = { headers }

        const token = await getToken({
          req,
          secret: process.env.AUTH_SECRET,
          secureCookie: process.env.NODE_ENV === 'production',
          salt:
            process.env.NODE_ENV === 'production'
              ? '__Secure-authjs.session-token'
              : 'authjs.session-token',
        })

        // Only clear cookie if NextAuth session is expired
        if (!token) {
          response.cookies.delete(BACKEND_JWT_COOKIE_NAME)
        }
      } catch {
        // If we can't check the token, don't clear the cookie (safer)
      }
    }
  } else {
    // For skipped routes, still check if NextAuth session is expired to clear stale backend JWT cookie
    // This is important for the home page ('/') so users can log in again when cookies expire
    try {
      const { getToken } = await import('next-auth/jwt')
      const headers = new Headers({
        cookie: request.headers.get('cookie') || '',
      })
      const req: { headers: Headers } = { headers }

      const token = await getToken({
        req,
        secret: process.env.AUTH_SECRET,
        secureCookie: process.env.NODE_ENV === 'production',
        salt:
          process.env.NODE_ENV === 'production'
            ? '__Secure-authjs.session-token'
            : 'authjs.session-token',
      })

      // If NextAuth session is expired and we have a backend JWT cookie, clear it
      if (!token && request.cookies.get(BACKEND_JWT_COOKIE_NAME)) {
        response.cookies.delete(BACKEND_JWT_COOKIE_NAME)
      }
    } catch {
      // Silently ignore errors when checking token on skipped routes
    }
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
