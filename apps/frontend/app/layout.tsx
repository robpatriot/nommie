// apps/frontend/app/layout.tsx
import type { Metadata } from 'next'
import { Inter } from 'next/font/google'
import { Suspense } from 'react'
import Script from 'next/script'
import { cookies } from 'next/headers'
import './globals.css'
import Header from '@/components/Header'
import { signOut } from '@/auth'
import { getLastActiveGame } from '@/lib/api'
import {
  resolveBackendJwt,
  BackendJwtResolution,
} from '@/lib/server/get-backend-jwt'
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
  let session: Session | null = null
  let backendJwt: string | undefined

  const resolution: BackendJwtResolution = await resolveBackendJwt()

  if (resolution.state === 'missing-session') {
    session = null
  } else if (resolution.state === 'missing-jwt') {
    await signOut({ redirectTo: '/' })
  } else {
    session = resolution.session
    backendJwt = resolution.backendJwt
  }

  // Only try to get last active game if we have a backend JWT
  // This prevents 401 errors when the token is missing or expired
  let lastActiveGameId: number | null = null

  if (backendJwt) {
    try {
      lastActiveGameId = await getLastActiveGame()
    } catch {
      // Silently handle errors - the endpoint might not exist or token might be expired
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
      <body
        className={`${inter.className} min-h-screen bg-background text-foreground antialiased`}
      >
        <Script id="theme-sync" strategy="beforeInteractive">
          {themeScript}
        </Script>
        <ThemeProvider
          initialTheme={initialTheme}
          initialResolved={initialResolved}
        >
          <Suspense fallback={null}>
            <Header session={session} lastActiveGameId={lastActiveGameId} />
          </Suspense>
          {children}
        </ThemeProvider>
      </body>
    </html>
  )
}
