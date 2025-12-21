import { NextResponse } from 'next/server'
import type { NextRequest } from 'next/server'
import { ensureBackendJwtForMiddleware } from '@/lib/auth/refresh-backend-jwt'
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
  const { locale, source } = resolveLocale({
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

    console.info('locale.resolved', {
      locale,
      source,
    })
  }

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
    return response
  }

  // For routes that need backend auth, ensure JWT is valid
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
