import { NextIntlClientProvider } from 'next-intl'
import { cookies, headers } from 'next/headers'

import { LOCALE_COOKIE_NAME, resolveLocale } from '@/i18n/locale'
import { loadMessages } from '@/i18n/messages'

export default async function SettingsLayout({
  children,
}: {
  children: React.ReactNode
}) {
  const cookieStore = await cookies()
  const headerStore = await headers()

  const { locale } = resolveLocale({
    cookieLocale: cookieStore.get(LOCALE_COOKIE_NAME)?.value ?? null,
    acceptLanguageHeader: headerStore.get('accept-language'),
  })

  const messages = await loadMessages(locale, ['settings'])

  return (
    <NextIntlClientProvider locale={locale} messages={messages}>
      {children}
    </NextIntlClientProvider>
  )
}
