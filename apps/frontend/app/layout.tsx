// apps/frontend/app/layout.tsx
import type { Metadata } from 'next'
import { Inter } from 'next/font/google'
import { Suspense } from 'react'
import './globals.css'
import Header from '@/components/Header'
import { auth } from '@/auth'
import { getLastActiveGame } from '@/lib/api'
import { getBackendJwtServer } from '@/lib/server/get-backend-jwt'

const inter = Inter({ subsets: ['latin'] })

export const metadata: Metadata = {
  title: 'Nommie',
  description: 'Web-based multiplayer version of Nomination Whist',
}

export default async function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  const session = await auth()
  // Only try to get last active game if we have a backend JWT
  // This prevents 401 errors when the token is missing or expired
  let lastActiveGameId: number | null = null
  const backendJwt = await getBackendJwtServer()
  if (backendJwt) {
    try {
      lastActiveGameId = await getLastActiveGame()
    } catch {
      // Silently handle errors - the endpoint might not exist or token might be expired
      // The header will just not show the resume button
    }
  }
  return (
    <html lang="en">
      <body className={inter.className}>
        <Suspense fallback={null}>
          <Header session={session} lastActiveGameId={lastActiveGameId} />
        </Suspense>
        {children}
      </body>
    </html>
  )
}
