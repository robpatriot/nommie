// apps/frontend/app/layout.tsx
import type { Metadata } from 'next'
import { Inter } from 'next/font/google'
import { Suspense } from 'react'
import Script from 'next/script'
import { cookies } from 'next/headers'
import './globals.css'
import Header from '@/components/Header'
import { HeaderBreadcrumbProvider } from '@/components/header-breadcrumbs'
import { getLastActiveGame } from '@/lib/api'
import { auth } from '@/auth'
import type { Session } from 'next-auth'

import {
  ThemeProvider,
  type ThemeMode,
  type ResolvedTheme,
} from '@/components/theme-provider'

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

  // Try to get last active game (will refresh JWT if needed)
  let lastActiveGameId: number | null = null
  if (session) {
    try {
      lastActiveGameId = await getLastActiveGame()
    } catch {
      // Silently handle errors - the endpoint might not exist or JWT might be missing
      // The header will just not show the resume button
    }
  }

  const cookieStore = await cookies()
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

  return (
    <html
      lang="en"
      data-theme={initialResolved}
      data-user-theme={initialTheme}
      suppressHydrationWarning
    >
      <body className={`${inter.className} tabletop-shell`}>
        <Script id="theme-sync" strategy="beforeInteractive">
          {themeScript}
        </Script>
        <ThemeProvider
          initialTheme={initialTheme}
          initialResolved={initialResolved}
        >
          <HeaderBreadcrumbProvider>
            <div className="tabletop-content">
              <Suspense fallback={null}>
                <Header session={session} lastActiveGameId={lastActiveGameId} />
              </Suspense>
              {children}
            </div>
          </HeaderBreadcrumbProvider>
        </ThemeProvider>
      </body>
    </html>
  )
}
