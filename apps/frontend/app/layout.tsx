// apps/frontend/app/layout.tsx
import type { Metadata } from 'next'
import { Inter } from 'next/font/google'
import { Suspense } from 'react'
import Script from 'next/script'
import { cookies, headers } from 'next/headers'
import './globals.css'
import Header from '@/components/Header'
import { HeaderBreadcrumbProvider } from '@/components/header-breadcrumbs'
import { auth } from '@/auth'
import type { Session } from 'next-auth'
import { NextIntlClientProvider } from 'next-intl'

import {
  ThemeProvider,
  type ThemeMode,
  type ResolvedTheme,
} from '@/components/theme-provider'
import PerformanceMonitorWrapper from '@/components/PerformanceMonitorWrapper'
import { AppQueryClientProvider } from '@/lib/providers/query-client-provider'
import { LOCALE_COOKIE_NAME, resolveLocale } from '@/i18n/locale'
import { loadMessages } from '@/i18n/messages'

const inter = Inter({ subsets: ['latin'] })

const themeScript = `
(() => {
  try {
    const storageKey = 'nommie.theme';
    const stored = localStorage.getItem(storageKey);
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const prefersDark = media.matches;
    const theme = stored === 'light' || stored === 'dark' ? stored : 'system';
    const resolved = theme === 'system' ? (prefersDark ? 'dark' : 'light') : theme;
    const root = document.documentElement;
    root.dataset.theme = resolved;
    root.dataset.userTheme = theme;
    root.classList.toggle('dark', resolved === 'dark');
    root.style.colorScheme = resolved;
  } catch (error) {
    // no-op
  }
})();
`

export const metadata: Metadata = {
  title: 'Nommie',
  description: 'Web-based multiplayer version of Nomination Whist',
}

export default async function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  const session: Session | null = await auth()

  const cookieStore = await cookies()
  const headerStore = await headers()

  const { locale } = resolveLocale({
    cookieLocale: cookieStore.get(LOCALE_COOKIE_NAME)?.value ?? null,
    acceptLanguageHeader: headerStore.get('accept-language'),
  })

  const themeCookie = cookieStore.get('nommie_theme')?.value ?? null

  let initialTheme: ThemeMode = 'system'
  let initialResolved: ResolvedTheme = 'light'

  if (themeCookie === 'light' || themeCookie === 'dark') {
    initialTheme = themeCookie
    initialResolved = themeCookie
  } else if (themeCookie?.startsWith('system:')) {
    const [, suffix] = themeCookie.split(':')
    if (suffix === 'dark' || suffix === 'light') {
      initialTheme = 'system'
      initialResolved = suffix
    }
  }

  const messages = await loadMessages(locale, [
    'common',
    'nav',
    'errors',
    'toasts',
    'lobby',
    'guide',
  ])

  return (
    <html
      lang={locale}
      data-theme={initialResolved}
      data-user-theme={initialTheme}
      suppressHydrationWarning
    >
      <body className={`${inter.className} tabletop-shell`}>
        <Script id="theme-sync" strategy="beforeInteractive">
          {themeScript}
        </Script>
        <NextIntlClientProvider locale={locale} messages={messages}>
          <ThemeProvider
            initialTheme={initialTheme}
            initialResolved={initialResolved}
          >
            <AppQueryClientProvider>
              <PerformanceMonitorWrapper />
              <HeaderBreadcrumbProvider>
                <div className="tabletop-content">
                  <Suspense fallback={null}>
                    <Header session={session} />
                  </Suspense>
                  {children}
                </div>
              </HeaderBreadcrumbProvider>
            </AppQueryClientProvider>
          </ThemeProvider>
        </NextIntlClientProvider>
      </body>
    </html>
  )
}
