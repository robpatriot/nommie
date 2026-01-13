// apps/frontend/app/layout.tsx
import type { Metadata } from 'next'
import {
  Inter,
  Righteous,
  Bebas_Neue,
  Playfair_Display,
} from 'next/font/google'
import { Suspense } from 'react'
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

// Boot script: server DOM is authoritative, localStorage is only fallback.
// Resolves "system" for first paint by toggling <html class="dark"> + color-scheme.
const themeScript = `
(() => {
  try {
    const COLOUR_SCHEME_STORAGE_KEY = 'nommie.colour_scheme';
    const THEME_NAME_STORAGE_KEY = 'nommie.theme_name';

    const VALID_THEME_NAMES = ['standard', 'high_roller', 'oldtime'];
    const VALID_COLOUR_SCHEMES = ['light', 'dark', 'system'];

    const isValidThemeName = (value) =>
      typeof value === 'string' && VALID_THEME_NAMES.includes(value);

    const isValidColourScheme = (value) =>
      typeof value === 'string' && VALID_COLOUR_SCHEMES.includes(value);

    const root = document.documentElement;

    // Theme name: prefer server; fall back to storage; then default.
    const serverThemeName = root.dataset.themeName;
    const storedThemeName = localStorage.getItem(THEME_NAME_STORAGE_KEY);
    const themeName = isValidThemeName(serverThemeName)
      ? serverThemeName
      : (isValidThemeName(storedThemeName) ? storedThemeName : 'standard');

    if (root.dataset.themeName !== themeName) root.dataset.themeName = themeName;

    // Colour scheme preference: prefer server; fall back to storage; then default.
    const serverColourScheme = root.dataset.colourScheme;
    const storedColourScheme = localStorage.getItem(COLOUR_SCHEME_STORAGE_KEY);
    const colourScheme = isValidColourScheme(serverColourScheme)
      ? serverColourScheme
      : (isValidColourScheme(storedColourScheme) ? storedColourScheme : 'system');

    if (root.dataset.colourScheme !== colourScheme) root.dataset.colourScheme = colourScheme;

    // Resolve system for first paint visuals.
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    const resolved = colourScheme === 'system'
      ? (prefersDark ? 'dark' : 'light')
      : colourScheme;

    const shouldBeDark = resolved === 'dark';
    if (root.classList.contains('dark') !== shouldBeDark) root.classList.toggle('dark', shouldBeDark);
    if (root.style.colorScheme !== resolved) root.style.colorScheme = resolved;
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
  let initialThemeName: ThemeName = 'standard'

  // Backend is authoritative for logged-in users
  if (session) {
    try {
      const options = await getUserOptions()
      initialThemeName = options.theme
      initialColourScheme = options.colour_scheme
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

  // Only set server-side "dark" class (and color-scheme) when user explicitly chose
  // light/dark. For "system", we let the boot script resolve on the client.
  const serverResolved: ResolvedColourScheme | null =
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
      style={serverResolved ? { colorScheme: serverResolved } : undefined}
      suppressHydrationWarning
    >
      <head>
        <script
          id="theme-sync"
          dangerouslySetInnerHTML={{ __html: themeScript }}
        />
      </head>

      <body
        className={`${inter.variable} ${righteous.variable} ${bebasNeue.variable} ${playfair.variable} font-sans tabletop-shell`}
      >
        <NextIntlClientProvider locale={locale} messages={messages}>
          <ThemeProvider
            initialColourScheme={initialColourScheme}
            initialThemeName={initialThemeName}
            isAuthenticated={!!session}
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
