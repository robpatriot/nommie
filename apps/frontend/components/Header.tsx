'use client'

import Link from 'next/link'
import ResumeGameButton from './ResumeGameButton'
import { ThemeToggle } from './theme-toggle'
import {
  signInWithGoogleAction,
  signOutAction,
} from '@/app/actions/auth-actions'

type HeaderProps = {
  session: { user?: { email?: string | null } } | null
  lastActiveGameId?: number | null
}

export default function Header({ session, lastActiveGameId }: HeaderProps) {
  // [AUTH_BYPASS] - Show lobby link when auth disabled
  const disableAuth = process.env.NEXT_PUBLIC_DISABLE_AUTH === 'true'
  const showLobbyLink = session?.user || disableAuth

  return (
    <header className="flex w-full items-center justify-between gap-3 border-b border-border bg-surface-strong px-4 py-4">
      <div className="flex items-center gap-4">
        <Link href="/" className="text-xl font-bold text-foreground">
          🃏 Nommie
        </Link>
        {showLobbyLink && (
          <Link
            href="/lobby"
            className="text-sm text-muted transition-colors hover:text-foreground hover:underline"
          >
            Lobby
          </Link>
        )}
      </div>
      <div className="flex items-center gap-3">
        {session?.user ? (
          <>
            <ThemeToggle />
            <ResumeGameButton lastActiveGameId={lastActiveGameId ?? null} />
            <span className="text-sm text-muted">{session.user.email}</span>
            <form action={signOutAction}>
              <button
                type="submit"
                className="rounded bg-surface px-3 py-1 text-sm text-foreground transition-colors hover:bg-surface-strong"
              >
                Sign out
              </button>
            </form>
          </>
        ) : (
          <>
            <ThemeToggle />
            <form action={signInWithGoogleAction}>
              <button
                type="submit"
                className="rounded bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground transition-colors hover:bg-primary/90"
              >
                Sign in with Google
              </button>
            </form>
          </>
        )}
      </div>
    </header>
  )
}
