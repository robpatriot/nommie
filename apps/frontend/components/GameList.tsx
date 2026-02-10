'use client'

import { useMemo, useState, type ReactNode } from 'react'
import { useLocale, useTranslations } from 'next-intl'
import { formatNumber } from '@/utils/number-formatting'
import type { Game } from '@/lib/types'
import { Card } from '@/components/ui/Card'

interface GameListProps {
  games: Game[]
  title: string
  emptyMessage: string
  actionsLabel?: string
  renderActions?: (game: Game) => ReactNode
}

const stateClassNames: Record<Game['state'], string> = {
  LOBBY: 'bg-success/15 text-success-foreground',
  BIDDING: 'bg-warning/15 text-warning-foreground',
  TRUMP_SELECTION: 'bg-warning/15 text-warning-foreground',
  TRICK_PLAY: 'bg-accent/20 text-accent-foreground',
  SCORING: 'bg-primary/15 text-primary',
  COMPLETED: 'bg-muted/15 text-muted-foreground',
  ABANDONED: 'bg-destructive/15 text-destructive-foreground',
}

export default function GameList({
  games,
  title,
  emptyMessage,
  actionsLabel,
  renderActions,
}: GameListProps) {
  const locale = useLocale()
  const t = useTranslations('lobby.gameList')
  const tLobby = useTranslations('lobby')
  const [searchQuery, setSearchQuery] = useState('')
  const [isMounted] = useState(true)

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
    <Card as="section" padding="md">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <h2 className="text-2xl font-semibold text-foreground">{title}</h2>
        {games.length > 0 ? (
          <label className="flex w-full items-center gap-2 rounded-2xl border border-border/60 bg-card px-3 py-2 text-sm text-muted-foreground shadow-inner shadow-shadow/10 sm:w-64">
            <span role="img" aria-hidden>
              🔍
            </span>
            <span className="sr-only">{t('search.ariaLabel')}</span>
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t('search.placeholder')}
              className="w-full bg-transparent text-sm text-foreground placeholder:text-muted-foreground focus-visible:outline-none"
            />
          </label>
        ) : null}
      </div>

      {games.length === 0 ? (
        <div className="mt-6 rounded-2xl border border-dashed border-border/60 bg-card px-4 py-8 text-center text-muted-foreground">
          {emptyMessage}
        </div>
      ) : filteredGames.length === 0 ? (
        <div className="mt-6 rounded-2xl border border-dashed border-border/60 bg-card px-4 py-8 text-center text-muted-foreground">
          {t('noMatches')}
        </div>
      ) : (
        <div className="mt-6 grid gap-4">
          {filteredGames.map((game) => {
            const stateLabel = getStateLabel(game.state)
            const stateClass =
              stateClassNames[game.state] ?? 'bg-card text-muted-foreground'
            const actions = renderActions?.(game)
            const relativeUpdated = isMounted
              ? formatRelativeTime(game.updated_at)
              : ''
            const seatsOpen = formatNumber(
              Math.max(game.max_players - game.player_count, 0),
              locale
            )

            return (
              <article
                key={game.id}
                className="group card-hover-shadow rounded-2xl border border-border/60 bg-card px-4 py-5 transition-transform duration-200 hover:-translate-y-1 hover:border-primary/40"
              >
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <p className="text-xs uppercase tracking-[0.4em] text-muted-foreground">
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

                <dl className="mt-4 grid gap-3 text-sm text-muted-foreground sm:grid-cols-3">
                  <div className="rounded-2xl bg-muted/70 px-3 py-2">
                    <dt className="text-xs uppercase tracking-wide text-muted-foreground">
                      {t('fields.players')}
                    </dt>
                    <dd className="text-base font-semibold text-foreground">
                      {formatNumber(game.player_count, locale)} /{' '}
                      {formatNumber(game.max_players, locale)}
                    </dd>
                  </div>
                  <div className="rounded-2xl bg-muted/70 px-3 py-2">
                    <dt className="text-xs uppercase tracking-wide text-muted-foreground">
                      {t('fields.seatsOpen')}
                    </dt>
                    <dd className="text-base font-semibold text-foreground">
                      {seatsOpen}
                    </dd>
                  </div>
                  <div className="rounded-2xl bg-muted/70 px-3 py-2">
                    <dt className="text-xs uppercase tracking-wide text-muted-foreground">
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
    </Card>
  )
}
