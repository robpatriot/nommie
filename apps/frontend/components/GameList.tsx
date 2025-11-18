'use client'

import { useMemo, useState, type ReactNode } from 'react'
import type { Game } from '@/lib/types'

interface GameListProps {
  games: Game[]
  title: string
  emptyMessage: string
  actionsLabel?: string
  renderActions?: (game: Game) => ReactNode
}

const stateLabels: Record<Game['state'], string> = {
  LOBBY: 'Lobby',
  DEALING: 'Dealing',
  BIDDING: 'Bidding',
  TRUMP_SELECTION: 'Picking trump',
  TRICK_PLAY: 'Trick play',
  SCORING: 'Scoring',
  BETWEEN_ROUNDS: 'Between rounds',
  COMPLETED: 'Completed',
  ABANDONED: 'Abandoned',
}

const stateClassNames: Record<Game['state'], string> = {
  LOBBY: 'bg-success/15 text-success-contrast',
  DEALING: 'bg-accent/15 text-accent-contrast',
  BIDDING: 'bg-warning/15 text-warning-foreground',
  TRUMP_SELECTION: 'bg-warning/15 text-warning-foreground',
  TRICK_PLAY: 'bg-accent/20 text-accent-contrast',
  SCORING: 'bg-primary/15 text-primary-foreground/80',
  BETWEEN_ROUNDS: 'bg-muted/15 text-subtle',
  COMPLETED: 'bg-muted/15 text-subtle',
  ABANDONED: 'bg-danger/15 text-danger-foreground',
}

const formatRelativeTime = (value: string) => {
  const timestamp = Date.parse(value)
  if (Number.isNaN(timestamp)) {
    return 'Unknown'
  }

  const diff = Date.now() - timestamp
  const minutes = Math.round(diff / (1000 * 60))
  if (minutes < 1) return 'just now'
  if (minutes < 60) return `${minutes}m ago`
  const hours = Math.round(minutes / 60)
  if (hours < 24) return `${hours}h ago`
  const days = Math.round(hours / 24)
  return `${days}d ago`
}

export default function GameList({
  games,
  title,
  emptyMessage,
  actionsLabel = 'Actions',
  renderActions,
}: GameListProps) {
  const [searchQuery, setSearchQuery] = useState('')

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
    <section className="rounded-3xl border border-white/10 bg-surface/80 p-5 shadow-[0_30px_90px_rgba(0,0,0,0.3)] backdrop-blur">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <h2 className="text-2xl font-semibold text-foreground">{title}</h2>
        {games.length > 0 ? (
          <label className="flex w-full items-center gap-2 rounded-2xl border border-border/60 bg-surface px-3 py-2 text-sm text-muted shadow-inner shadow-black/10 sm:w-64">
            <span role="img" aria-hidden>
              🔍
            </span>
            <span className="sr-only">Search games</span>
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search by name or ID"
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
          No games match your search
        </div>
      ) : (
        <div className="mt-6 grid gap-4">
          {filteredGames.map((game) => {
            const stateLabel = stateLabels[game.state] ?? game.state
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
                      Game #{game.id}
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
                      Players
                    </dt>
                    <dd className="text-base font-semibold text-foreground">
                      {game.player_count} / {game.max_players}
                    </dd>
                  </div>
                  <div className="rounded-2xl bg-surface-strong/70 px-3 py-2">
                    <dt className="text-xs uppercase tracking-wide text-subtle">
                      Seats open
                    </dt>
                    <dd className="text-base font-semibold text-foreground">
                      {seatsOpen}
                    </dd>
                  </div>
                  <div className="rounded-2xl bg-surface-strong/70 px-3 py-2">
                    <dt className="text-xs uppercase tracking-wide text-subtle">
                      Updated
                    </dt>
                    <dd className="text-base font-semibold text-foreground">
                      {relativeUpdated}
                    </dd>
                  </div>
                </dl>

                {showActions && actions ? (
                  <div
                    className="mt-5 flex flex-wrap gap-2 text-sm"
                    aria-label={actionsLabel}
                  >
                    {actions}
                  </div>
                ) : null}
              </article>
            )
          })}
        </div>
      )}
    </section>
  )
}
