'use client'

import Link from 'next/link'
import { useEffect, useRef, useState } from 'react'
import { useTranslations } from 'next-intl'
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

type HeaderProps = {
  session: { user?: { email?: string | null } } | null
  lastActiveGameId?: number | null
}

export default function Header({ session, lastActiveGameId }: HeaderProps) {
  const t = useTranslations('nav')
  const { crumbs } = useHeaderBreadcrumbs()
  const hasBreadcrumbs = session?.user && crumbs.length > 0
  const [isUserMenuOpen, setIsUserMenuOpen] = useState(false)
  const userMenuRef = useRef<HTMLDivElement | null>(null)

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

  return (
    <header className="sticky top-0 z-30 border-b border-white/10 bg-surface-strong/70 px-3 py-3 shadow-[0_20px_60px_rgba(0,0,0,0.25)] backdrop-blur-lg">
      <div className="mx-auto flex w-full max-w-6xl flex-row items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <Link
            href="/"
            className="inline-flex items-center gap-3 rounded-full bg-surface px-3 py-2 text-lg font-semibold text-foreground shadow-inner shadow-black/10 transition hover:bg-surface-strong/80"
            aria-label={t('brand.homeAria')}
          >
            <span className="text-2xl" role="img" aria-hidden>
              🃏
            </span>
            <span className="hidden tracking-tight sm:inline">Nommie</span>
          </Link>
          {hasBreadcrumbs ? (
            <nav
              className="hidden items-center gap-2 text-sm text-muted sm:flex"
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
                      <span aria-hidden className="text-muted">
                        /
                      </span>
                    ) : null}
                    {crumb.href && !isLast ? (
                      <Link
                        href={crumb.href}
                        className="rounded-full px-3 py-1.5 font-medium text-muted transition hover:bg-surface/80 hover:text-foreground"
                      >
                        {crumb.label}
                      </Link>
                    ) : (
                      <span className="rounded-full bg-surface px-3 py-1 text-xs font-semibold uppercase tracking-wide text-subtle">
                        {crumb.label}
                      </span>
                    )}
                  </div>
                )
              })}
            </nav>
          ) : (
            <span className="text-sm text-muted">{t('tagline')}</span>
          )}
        </div>

        <div className="relative flex flex-wrap items-center gap-2 sm:justify-end">
          {session?.user ? (
            <>
              <ResumeGameButton
                lastActiveGameId={lastActiveGameId ?? null}
                className="bg-primary/90 px-4 py-2 text-sm font-semibold text-primary-foreground shadow-lg shadow-primary/30 hover:bg-primary"
              />
              <div className="relative" ref={userMenuRef}>
                <button
                  type="button"
                  onClick={() => setIsUserMenuOpen((open) => !open)}
                  className="inline-flex h-9 w-9 items-center justify-center rounded-full border border-border/60 bg-surface text-sm font-semibold uppercase tracking-wide text-muted transition hover:border-primary/50 hover:text-foreground"
                  aria-haspopup="true"
                  aria-expanded={isUserMenuOpen}
                  aria-label={t('account.menuAria')}
                >
                  {getInitials(session.user.email)}
                </button>
                {isUserMenuOpen ? (
                  <div className="absolute right-0 top-full mt-2 w-56 rounded-2xl border border-border/60 bg-surface p-3 text-sm shadow-lg shadow-black/20">
                    <p className="mb-2 truncate text-xs uppercase tracking-wide text-subtle">
                      {t('account.signedInAs')}
                    </p>
                    <p className="mb-3 truncate text-foreground">
                      {session.user.email}
                    </p>
                    <Link
                      href="/settings"
                      className="mb-2 flex w-full items-center justify-between rounded-2xl border border-border/70 bg-surface px-4 py-2 font-semibold text-foreground transition hover:border-primary/50 hover:text-primary"
                      onClick={() => setIsUserMenuOpen(false)}
                    >
                      {t('account.settings')}
                      <span aria-hidden>⚙️</span>
                    </Link>
                    <form action={signOutAction}>
                      <button
                        type="submit"
                        className="w-full rounded-2xl border border-border/70 bg-surface px-4 py-2 text-sm font-semibold text-foreground transition hover:border-primary/50 hover:text-primary"
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
