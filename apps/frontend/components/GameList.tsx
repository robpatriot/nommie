'use client'

import { useMemo, useState, type ReactNode } from 'react'
import { useTranslations } from 'next-intl'
import type { Game } from '@/lib/types'
import { SurfaceCard } from './SurfaceCard'

interface GameListProps {
  games: Game[]
  title: string
  emptyMessage: string
  actionsLabel?: string
  renderActions?: (game: Game) => ReactNode
}

const stateClassNames: Record<Game['state'], string> = {
  LOBBY: 'bg-success/15 text-success-contrast',
  DEALING: 'bg-accent/15 text-accent-contrast',
  BIDDING: 'bg-warning/15 text-warning-contrast',
  TRUMP_SELECTION: 'bg-warning/15 text-warning-contrast',
  TRICK_PLAY: 'bg-accent/20 text-accent-contrast',
  SCORING: 'bg-primary/15 text-primary-contrast',
  BETWEEN_ROUNDS: 'bg-muted/15 text-subtle',
  COMPLETED: 'bg-muted/15 text-subtle',
  ABANDONED: 'bg-danger/15 text-danger-foreground',
}

export default function GameList({
  games,
  title,
  emptyMessage,
  actionsLabel,
  renderActions,
}: GameListProps) {
  const t = useTranslations('lobby.gameList')
  const tLobby = useTranslations('lobby')
  const [searchQuery, setSearchQuery] = useState('')

  const getStateLabel = (state: Game['state']): string => {
    return t(`gameStates.${state}`)
  }

  const formatRelativeTime = useMemo(() => {
    return (value: string): string => {
      const timestamp = Date.parse(value)
      if (Number.isNaN(timestamp)) {
        return t('time.unknown')
      }

      const now = Date.now()
      const diff = now - timestamp
      const minutes = Math.round(diff / (1000 * 60))
      if (minutes < 1) return t('time.justNow')
      if (minutes < 60) return t('time.minutesAgo', { minutes })
      const hours = Math.round(minutes / 60)
      if (hours < 24) return t('time.hoursAgo', { hours })
      const days = Math.round(hours / 24)
      return t('time.daysAgo', { days })
    }
  }, [t])

  const filteredGames = useMemo(() => {
    if (!searchQuery.trim()) return games

    const query = searchQuery.toLowerCase()
    return games.filter(
      (game) =>
        game.name.toLowerCase().includes(query) ||
        game.id.toString().includes(query)
    )
  }, [games, searchQuery])

  const showActions = typeof renderActions === 'function'

  return (
    <SurfaceCard
      as="section"
      padding="md"
      className="shadow-[0_30px_90px_rgba(0,0,0,0.3)]"
    >
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <h2 className="text-2xl font-semibold text-foreground">{title}</h2>
        {games.length > 0 ? (
          <label className="flex w-full items-center gap-2 rounded-2xl border border-border/60 bg-surface px-3 py-2 text-sm text-muted shadow-inner shadow-black/10 sm:w-64">
            <span role="img" aria-hidden>
              🔍
            </span>
            <span className="sr-only">{t('search.ariaLabel')}</span>
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t('search.placeholder')}
              className="w-full bg-transparent text-sm text-foreground placeholder:text-muted focus-visible:outline-none"
            />
          </label>
        ) : null}
      </div>

      {games.length === 0 ? (
        <div className="mt-6 rounded-2xl border border-dashed border-border/60 bg-surface px-4 py-8 text-center text-muted">
          {emptyMessage}
        </div>
      ) : filteredGames.length === 0 ? (
        <div className="mt-6 rounded-2xl border border-dashed border-border/60 bg-surface px-4 py-8 text-center text-muted">
          {t('noMatches')}
        </div>
      ) : (
        <div className="mt-6 grid gap-4">
          {filteredGames.map((game) => {
            const stateLabel = getStateLabel(game.state)
            const stateClass =
              stateClassNames[game.state] ?? 'bg-surface text-subtle'
            const actions = renderActions?.(game)
            const relativeUpdated = formatRelativeTime(game.updated_at)
            const seatsOpen = Math.max(game.max_players - game.player_count, 0)

            return (
              <article
                key={game.id}
                className="group rounded-2xl border border-border/60 bg-surface px-4 py-5 shadow-[0_20px_60px_rgba(0,0,0,0.25)] transition hover:-translate-y-0.5 hover:border-primary/40"
              >
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <p className="text-xs uppercase tracking-[0.4em] text-subtle">
                      Game {game.id}
                    </p>
                    <h3 className="text-xl font-semibold text-foreground">
                      {game.name}
                    </h3>
                  </div>
                  <span
                    className={`rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-wide ${stateClass}`}
                  >
                    {stateLabel}
                  </span>
                </div>

                <dl className="mt-4 grid gap-3 text-sm text-muted sm:grid-cols-3">
                  <div className="rounded-2xl bg-surface-strong/70 px-3 py-2">
                    <dt className="text-xs uppercase tracking-wide text-subtle">
                      {t('fields.players')}
                    </dt>
                    <dd className="text-base font-semibold text-foreground">
                      {game.player_count} / {game.max_players}
                    </dd>
                  </div>
                  <div className="rounded-2xl bg-surface-strong/70 px-3 py-2">
                    <dt className="text-xs uppercase tracking-wide text-subtle">
                      {t('fields.seatsOpen')}
                    </dt>
                    <dd className="text-base font-semibold text-foreground">
                      {seatsOpen}
                    </dd>
                  </div>
                  <div className="rounded-2xl bg-surface-strong/70 px-3 py-2">
                    <dt className="text-xs uppercase tracking-wide text-subtle">
                      {t('fields.updated')}
                    </dt>
                    <dd className="text-base font-semibold text-foreground">
                      {relativeUpdated}
                    </dd>
                  </div>
                </dl>

                {showActions && actions ? (
                  <div
                    className="mt-5 flex flex-wrap gap-2 text-sm"
                    aria-label={actionsLabel ?? tLobby('lists.actionsLabel')}
                  >
                    {actions}
                  </div>
                ) : null}
              </article>
            )
          })}
        </div>
      )}
    </SurfaceCard>
  )
}
