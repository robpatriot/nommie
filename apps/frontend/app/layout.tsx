// apps/frontend/app/layout.tsx
import type { Metadata } from 'next'
import { Inter } from 'next/font/google'
import { Suspense } from 'react'
import './globals.css'
import Header from '@/components/Header'
import { auth } from '@/auth'
import { getLastActiveGame } from '@/lib/api'

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
  const lastActiveGameId = session ? await getLastActiveGame() : null
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
