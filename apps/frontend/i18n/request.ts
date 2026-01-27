import { getRequestConfig } from 'next-intl/server'
import { cookies, headers } from 'next/headers'
import {
  LOCALE_COOKIE_NAME,
  resolveLocale,
  type SupportedLocale,
} from './locale'
import { loadMessages } from './messages'

export default getRequestConfig(async () => {
  const cookieStore = await cookies()
  const headerStore = await headers()

  // Priority order:
  // 1. Cookie (synced from backend preference when user updates locale)
  // 2. Accept-Language header
  // 3. Default locale
  const cookieLocale = cookieStore.get(LOCALE_COOKIE_NAME)?.value ?? null

  const { locale } = resolveLocale({
    cookieLocale,
    acceptLanguageHeader: headerStore.get('accept-language'),
  })

  const messages = await loadMessages(locale as SupportedLocale, [
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
