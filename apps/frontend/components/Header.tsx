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
  return (
    <header className="sticky top-0 z-30 border-b border-white/10 bg-surface-strong/70 px-3 py-3 shadow-[0_20px_60px_rgba(0,0,0,0.25)] backdrop-blur-lg">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-center gap-3">
          <Link
            href="/"
            className="inline-flex items-center gap-3 rounded-full bg-surface px-3 py-2 text-lg font-semibold text-foreground shadow-inner shadow-black/10 transition hover:bg-surface-strong/80"
            aria-label="Nommie home"
          >
            <span className="text-2xl" role="img" aria-hidden>
              🃏
            </span>
            <span className="tracking-tight">Nommie</span>
          </Link>
          {session?.user ? (
            <nav className="hidden items-center gap-2 text-sm text-muted sm:flex">
              <Link
                href="/lobby"
                className="rounded-full px-3 py-1.5 font-medium text-muted transition hover:bg-surface/80 hover:text-foreground"
              >
                Lobby
              </Link>
              <span aria-hidden className="text-muted">
                /
              </span>
              <span className="rounded-full bg-surface px-3 py-1 text-xs font-semibold uppercase tracking-wide text-subtle">
                Focused play
              </span>
            </nav>
          ) : (
            <span className="text-sm text-muted">
              A calm table for playing Nomination Whist
            </span>
          )}
        </div>

        <div className="flex flex-wrap items-center gap-2 sm:justify-end">
          <ThemeToggle className="bg-surface/80 text-foreground" />
          {session?.user ? (
            <>
              <ResumeGameButton
                lastActiveGameId={lastActiveGameId ?? null}
                className="bg-primary/90 px-4 py-2 text-sm font-semibold text-primary-foreground shadow-lg shadow-primary/30 hover:bg-primary"
              />
              <span className="rounded-full bg-surface px-3 py-1 text-xs font-medium uppercase tracking-wide text-muted">
                {session.user.email}
              </span>
              <form action={signOutAction}>
                <button
                  type="submit"
                  className="rounded-full border border-border/70 bg-surface px-4 py-2 text-sm font-semibold text-foreground transition hover:border-primary/50 hover:text-primary"
                >
                  Sign out
                </button>
              </form>
            </>
          ) : (
            <form action={signInWithGoogleAction} className="w-full sm:w-auto">
              <button
                type="submit"
                className="flex w-full items-center justify-center gap-2 rounded-full bg-primary px-5 py-2 text-sm font-semibold text-primary-foreground shadow-lg shadow-primary/30 transition hover:bg-primary/90"
              >
                <span role="img" aria-hidden>
                  ✨
                </span>
                Sign in with Google
              </button>
            </form>
          )}
        </div>
      </div>
    </header>
  )
}
