'use client'

import Link from 'next/link'
import { useEffect, useRef, useState } from 'react'
import { useTranslations } from 'next-intl'
import { usePathname } from 'next/navigation'

function getInitials(email?: string | null) {
  if (!email) {
    return '👤'
  }

  const [namePart] = email.split('@')
  if (!namePart) {
    return email.charAt(0).toUpperCase()
  }

  const parts = namePart
    .replace(/[._-]+/g, ' ')
    .split(' ')
    .filter(Boolean)

  if (parts.length >= 2) {
    return (parts[0][0] + parts[1][0]).toUpperCase()
  }

  return parts[0].slice(0, 2).toUpperCase()
}

import ResumeGameButton from './ResumeGameButton'
import {
  signInWithGoogleAction,
  signOutAction,
} from '@/app/actions/auth-actions'
import { useHeaderBreadcrumbs } from './header-breadcrumbs'
import { useWaitingLongestGame } from '@/hooks/queries/useGames'

type HeaderProps = {
  session: { user?: { email?: string | null } } | null
}

export default function Header({ session }: HeaderProps) {
  const t = useTranslations('nav')
  const pathname = usePathname()
  const { crumbs } = useHeaderBreadcrumbs()
  const hasBreadcrumbs = session?.user && crumbs.length > 0
  const [isUserMenuOpen, setIsUserMenuOpen] = useState(false)
  const [isBrandMenuOpen, setIsBrandMenuOpen] = useState(false)
  const userMenuRef = useRef<HTMLDivElement | null>(null)
  const brandMenuRef = useRef<HTMLDivElement | null>(null)

  // Detect current game ID for exclusion from "Resume Game" logic
  const gamePathMatch = pathname.match(/^\/game\/(\d+)/)
  const currentGameId = gamePathMatch
    ? parseInt(gamePathMatch[1], 10)
    : undefined

  // Fetch the game ID that has been waiting the longest for the user's attention.
  const { data: waitingGameId } = useWaitingLongestGame({
    enabled: !!session?.user,
    excludeGameId: currentGameId,
  })

  useEffect(() => {
    if (!isUserMenuOpen) {
      return
    }

    const handlePointerDown = (event: PointerEvent) => {
      if (!userMenuRef.current) {
        return
      }

      if (!userMenuRef.current.contains(event.target as Node)) {
        setIsUserMenuOpen(false)
      }
    }

    document.addEventListener('pointerdown', handlePointerDown)
    return () => document.removeEventListener('pointerdown', handlePointerDown)
  }, [isUserMenuOpen])

  useEffect(() => {
    if (!isBrandMenuOpen) {
      return
    }

    const handlePointerDown = (event: PointerEvent) => {
      if (!brandMenuRef.current) {
        return
      }

      if (!brandMenuRef.current.contains(event.target as Node)) {
        setIsBrandMenuOpen(false)
      }
    }

    document.addEventListener('pointerdown', handlePointerDown)
    return () => document.removeEventListener('pointerdown', handlePointerDown)
  }, [isBrandMenuOpen])

  const isOnLobby = pathname === '/lobby'
  const isOnGuide = pathname === '/guide'

  return (
    <header className="sticky top-0 z-30 border-b border-border/60 bg-muted/70 px-3 py-3 shadow-[0_8px_30px_rgba(0,0,0,0.15)] backdrop-blur-lg">
      <div className="mx-auto flex w-full max-w-6xl flex-row items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          {session?.user ? (
            <div className="relative" ref={brandMenuRef}>
              <button
                type="button"
                onClick={() => setIsBrandMenuOpen((open) => !open)}
                className="inline-flex items-center gap-2 rounded-full bg-card px-3 py-2 text-lg font-semibold text-foreground shadow-inner shadow-shadow/10 transition hover:bg-muted/80"
                aria-haspopup="true"
                aria-expanded={isBrandMenuOpen}
                aria-label={t('brand.menuAria')}
              >
                <span className="text-2xl" role="img" aria-hidden>
                  🃏
                </span>
                <span className="hidden tracking-tight sm:inline">Nommie</span>
                <span
                  className={`text-xs text-muted-foreground transition-transform ${
                    isBrandMenuOpen ? 'rotate-180' : ''
                  }`}
                  aria-hidden
                >
                  ▼
                </span>
              </button>
              {isBrandMenuOpen ? (
                <div className="absolute left-0 top-full mt-2 w-48 rounded-2xl border border-border/60 bg-card p-3 text-sm shadow-lg shadow-shadow/20">
                  <Link
                    href="/lobby"
                    className={`mb-2 flex w-full items-center justify-between rounded-2xl border border-border/70 bg-card px-4 py-2 font-semibold transition hover:border-primary/50 ${
                      isOnLobby
                        ? 'border-primary/50 text-primary'
                        : 'text-foreground hover:text-primary'
                    }`}
                    onClick={() => setIsBrandMenuOpen(false)}
                  >
                    {t('brand.lobby')}
                    {isOnLobby ? (
                      <span className="text-xs" aria-hidden>
                        ●
                      </span>
                    ) : null}
                  </Link>
                  <Link
                    href="/guide"
                    className={`flex w-full items-center justify-between rounded-2xl border border-border/70 bg-card px-4 py-2 font-semibold transition hover:border-primary/50 ${
                      isOnGuide
                        ? 'border-primary/50 text-primary'
                        : 'text-foreground hover:text-primary'
                    }`}
                    onClick={() => setIsBrandMenuOpen(false)}
                  >
                    {t('brand.guide')}
                    {isOnGuide ? (
                      <span className="text-xs" aria-hidden>
                        ●
                      </span>
                    ) : null}
                  </Link>
                </div>
              ) : null}
            </div>
          ) : (
            <Link
              href="/"
              className="inline-flex items-center gap-3 rounded-full bg-card px-3 py-2 text-lg font-semibold text-foreground shadow-inner shadow-shadow/10 transition hover:bg-muted/80"
              aria-label={t('brand.homeAria')}
            >
              <span className="text-2xl" role="img" aria-hidden>
                🃏
              </span>
              <span className="hidden tracking-tight sm:inline">Nommie</span>
            </Link>
          )}
          {hasBreadcrumbs ? (
            <nav
              className="hidden items-center gap-2 text-sm text-muted-foreground sm:flex"
              aria-label={t('breadcrumbs.ariaLabel')}
            >
              {crumbs.map((crumb, index) => {
                const isLast = index === crumbs.length - 1
                return (
                  <div
                    key={`${crumb.label}-${index}`}
                    className="flex items-center gap-2"
                  >
                    {index > 0 ? (
                      <span aria-hidden className="text-muted-foreground">
                        /
                      </span>
                    ) : null}
                    {crumb.href && !isLast ? (
                      <Link
                        href={crumb.href}
                        className="rounded-full px-3 py-1.5 font-medium text-muted-foreground transition hover:bg-card/80 hover:text-foreground"
                      >
                        {crumb.label}
                      </Link>
                    ) : (
                      <span className="rounded-full bg-card px-3 py-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                        {crumb.label}
                      </span>
                    )}
                  </div>
                )
              })}
            </nav>
          ) : (
            <span className="text-sm text-muted-foreground">
              {t('tagline')}
            </span>
          )}
        </div>

        <div className="relative flex flex-wrap items-center gap-2 sm:justify-end">
          {session?.user ? (
            <>
              <ResumeGameButton
                waitingGameId={waitingGameId ?? null}
                className="bg-primary/90 px-4 py-2 text-sm font-semibold text-primary-foreground shadow-lg shadow-primary/30 hover:bg-primary"
              />
              <div className="relative" ref={userMenuRef}>
                <button
                  type="button"
                  onClick={() => setIsUserMenuOpen((open) => !open)}
                  className="inline-flex h-9 w-9 items-center justify-center rounded-full border border-border/60 bg-card text-sm font-semibold uppercase tracking-wide text-muted-foreground transition hover:border-primary/50 hover:text-foreground"
                  aria-haspopup="true"
                  aria-expanded={isUserMenuOpen}
                  aria-label={t('account.menuAria')}
                >
                  {getInitials(session.user.email)}
                </button>
                {isUserMenuOpen ? (
                  <div className="absolute right-0 top-full mt-2 w-56 rounded-2xl border border-border/60 bg-card p-3 text-sm shadow-lg shadow-shadow/20">
                    <p className="mb-2 truncate text-xs uppercase tracking-wide text-muted-foreground">
                      {t('account.signedInAs')}
                    </p>
                    <p className="mb-3 truncate text-foreground">
                      {session.user.email}
                    </p>
                    <Link
                      href="/settings"
                      className="mb-2 flex w-full items-center justify-between rounded-2xl border border-border/70 bg-card px-4 py-2 font-semibold text-foreground transition hover:border-primary/50 hover:text-primary"
                      onClick={() => setIsUserMenuOpen(false)}
                    >
                      {t('account.settings')}
                      <span aria-hidden>⚙️</span>
                    </Link>
                    <form action={signOutAction}>
                      <button
                        type="submit"
                        className="w-full rounded-2xl border border-border/70 bg-card px-4 py-2 text-sm font-semibold text-foreground transition hover:border-primary/50 hover:text-primary"
                      >
                        {t('account.signOut')}
                      </button>
                    </form>
                  </div>
                ) : null}
              </div>
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
                {t('auth.signInWithGoogle')}
              </button>
            </form>
          )}
        </div>
      </div>
    </header>
  )
}
