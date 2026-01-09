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
    const storageKey = 'nommie.colour_scheme';
    const themeNameKey = 'nommie.theme_name';
    const stored = localStorage.getItem(storageKey);
    const storedThemeName = localStorage.getItem(themeNameKey);
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const prefersDark = media.matches;
    const colourScheme = stored === 'light' || stored === 'dark' ? stored : 'system';
    const resolved = colourScheme === 'system' ? (prefersDark ? 'dark' : 'light') : colourScheme;
    const root = document.documentElement;
    const serverThemeName = root.dataset.themeName;
    const isValidThemeName = (value) => value === 'standard' || value === 'high_roller' || value === 'oldtime';
    const themeName = isValidThemeName(storedThemeName) ? storedThemeName : (isValidThemeName(serverThemeName) ? serverThemeName : 'standard');
    root.dataset.themeName = themeName;
    root.dataset.colourScheme = colourScheme;
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

  const colourSchemeCookie =
    cookieStore.get('nommie_colour_scheme')?.value ?? null

  let initialColourScheme: ColourScheme = 'system'
  let initialResolved: ResolvedColourScheme = 'light'
  let initialThemeName: ThemeName = 'standard'

  if (colourSchemeCookie === 'light' || colourSchemeCookie === 'dark') {
    initialColourScheme = colourSchemeCookie
    initialResolved = colourSchemeCookie
  } else if (colourSchemeCookie?.startsWith('system:')) {
    const [, suffix] = colourSchemeCookie.split(':')
    if (suffix === 'dark' || suffix === 'light') {
      initialColourScheme = 'system'
      initialResolved = suffix
    }
  }

  // Fetch theme preference from backend if user is authenticated
  if (session) {
    try {
      const options = await getUserOptions()
      initialThemeName = options.theme
    } catch {
      // Fall back to default theme on error
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
      data-theme-name={initialThemeName}
      data-colour-scheme={initialColourScheme}
      suppressHydrationWarning
    >
      <body
        className={`${inter.variable} ${righteous.variable} ${bebasNeue.variable} ${playfair.variable} font-sans tabletop-shell`}
      >
        <Script id="theme-sync" strategy="beforeInteractive">
          {themeScript}
        </Script>
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
