import { getRequestConfig } from 'next-intl/server'
import { cookies, headers } from 'next/headers'
import { auth } from '@/auth'
import {
  LOCALE_COOKIE_NAME,
  resolveLocale,
  isSupportedLocale,
  type SupportedLocale,
} from './locale'
import { loadMessages } from './messages'
import { getUserOptions } from '@/lib/api/user-options'

export default getRequestConfig(async () => {
  const cookieStore = await cookies()
  const headerStore = await headers()
  const session = await auth()

  // Priority order:
  // 1. Backend user options (if authenticated)
  // 2. Cookie
  // 3. Accept-Language header
  // 4. Default locale

  let backendLocale: SupportedLocale | null = null
  if (session) {
    try {
      const options = await getUserOptions()
      if (options.locale && isSupportedLocale(options.locale)) {
        backendLocale = options.locale
      }
    } catch {
      // Silently handle errors - user might not have options set yet
      // or there might be an auth issue
    }
  }

  const cookieLocale = cookieStore.get(LOCALE_COOKIE_NAME)?.value ?? null

  // If backend has a locale preference, use it (and sync cookie if needed)
  let locale: SupportedLocale
  if (backendLocale) {
    locale = backendLocale
    // If cookie doesn't match backend preference, we'll need to update it
    // but we can't set cookies in getRequestConfig, so we'll handle this in proxy.ts
  } else {
    // Fall back to cookie/header resolution
    const resolved = resolveLocale({
      cookieLocale,
      acceptLanguageHeader: headerStore.get('accept-language'),
    })
    locale = resolved.locale
  }

  const messages = await loadMessages(locale, [
    'common',
    'nav',
    'settings',
    'errors',
    'toasts',
    'lobby',
    'game',
    'guide',
  ])

  return {
    locale,
    messages,
  }
})
