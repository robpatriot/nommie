// apps/frontend/app/layout.tsx
import type { Metadata } from 'next'
import {
  Inter,
  Righteous,
  Bebas_Neue,
  Playfair_Display,
} from 'next/font/google'
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
  type ColourScheme,
  type ResolvedColourScheme,
  type ThemeName,
} from '@/components/theme-provider'
import PerformanceMonitorWrapper from '@/components/PerformanceMonitorWrapper'
import { AppQueryClientProvider } from '@/lib/providers/query-client-provider'
import { LOCALE_COOKIE_NAME, resolveLocale } from '@/i18n/locale'
import { loadMessages } from '@/i18n/messages'
import { getUserOptions } from '@/lib/api/user-options'

const inter = Inter({ subsets: ['latin'], variable: '--font-inter' })

const righteous = Righteous({
  weight: '400',
  subsets: ['latin'],
  variable: '--font-oldtime-heading',
})

const bebasNeue = Bebas_Neue({
  weight: '400',
  subsets: ['latin'],
  variable: '--font-oldtime-display',
})

const playfair = Playfair_Display({
  subsets: ['latin'],
  variable: '--font-oldtime-elegant',
})

const themeScript = `
(() => {
  try {
    const COLOUR_SCHEME_STORAGE_KEY = 'nommie.colour_scheme';
    const THEME_NAME_STORAGE_KEY = 'nommie.theme_name';

    const storedColourScheme = localStorage.getItem(COLOUR_SCHEME_STORAGE_KEY);
    const storedThemeName = localStorage.getItem(THEME_NAME_STORAGE_KEY);

    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const prefersDark = media.matches;

    const root = document.documentElement;

    const isValidThemeName = (value) =>
      value === 'standard' || value === 'high_roller' || value === 'oldtime';

    // Backend / server is authoritative
    const serverThemeName = root.dataset.themeName;
    const themeName = isValidThemeName(serverThemeName)
      ? serverThemeName
      : (isValidThemeName(storedThemeName) ? storedThemeName : 'standard');

    root.dataset.themeName = themeName;

    const isValidScheme = (value) =>
      value === 'light' || value === 'dark' || value === 'system';

    const serverColourScheme = root.dataset.colourScheme;
    const colourScheme = isValidScheme(serverColourScheme)
      ? serverColourScheme
      : (isValidScheme(storedColourScheme) ? storedColourScheme : 'system');

    root.dataset.colourScheme = colourScheme;

    const resolved =
      colourScheme === 'system'
        ? (prefersDark ? 'dark' : 'light')
        : colourScheme;

    root.classList.toggle('dark', resolved === 'dark');
    root.style.colorScheme = resolved;
  } catch {
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

  let initialColourScheme: ColourScheme = 'system'
  let initialResolved: ResolvedColourScheme = 'light'
  let initialThemeName: ThemeName = 'standard'

  // Fetch theme preference from backend if user is authenticated
  if (session) {
    try {
      const options = await getUserOptions()
      initialThemeName = options.theme
      initialColourScheme = options.colour_scheme
      initialResolved =
        initialColourScheme === 'dark'
          ? 'dark'
          : initialColourScheme === 'light'
            ? 'light'
            : 'light'
    } catch {
      // Fall back to defaults on error
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

  const serverResolved =
    initialColourScheme === 'dark'
      ? 'dark'
      : initialColourScheme === 'light'
        ? 'light'
        : null

  return (
    <html
      lang={locale}
      data-theme-name={initialThemeName}
      data-colour-scheme={initialColourScheme}
      className={serverResolved === 'dark' ? 'dark' : undefined}
      style={
        serverResolved
          ? ({ colorScheme: serverResolved } as React.CSSProperties)
          : undefined
      }
      suppressHydrationWarning
    >
      <head>
        <Script id="theme-sync" strategy="beforeInteractive">
          {themeScript}
        </Script>
      </head>

      <body
        className={`${inter.variable} ${righteous.variable} ${bebasNeue.variable} ${playfair.variable} font-sans tabletop-shell`}
      >
        <NextIntlClientProvider locale={locale} messages={messages}>
          <ThemeProvider
            initialColourScheme={initialColourScheme}
            initialResolved={initialResolved}
            initialThemeName={initialThemeName}
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
