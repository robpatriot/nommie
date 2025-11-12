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

export default function GameList({
  games,
  title,
  emptyMessage,
  actionsLabel = 'Actions',
  renderActions,
}: GameListProps) {
  const [searchQuery, setSearchQuery] = useState('')

  // Client-side filtering
  const filteredGames = useMemo(() => {
    if (!searchQuery.trim()) return games

    const query = searchQuery.toLowerCase()
    return games.filter(
      (game) =>
        game.name.toLowerCase().includes(query) ||
        game.id.toString().includes(query)
    )
  }, [games, searchQuery])

  if (games.length === 0) {
    return (
      <div className="mb-8">
        <h2 className="mb-4 text-xl font-semibold text-foreground">{title}</h2>
        <div className="rounded-lg border border-border bg-surface p-6 text-center">
          <p className="text-muted">{emptyMessage}</p>
        </div>
      </div>
    )
  }

  const showActions = typeof renderActions === 'function'

  return (
    <div className="mb-8">
      <div className="mb-4 flex items-center justify-between gap-3">
        <h2 className="text-xl font-semibold text-foreground">{title}</h2>
        <input
          type="text"
          placeholder="Search games..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="w-56 rounded border border-border bg-background px-3 py-1.5 text-sm text-foreground shadow-inner focus:outline-none focus:ring-2 focus:ring-primary"
        />
      </div>

      <div className="overflow-hidden rounded-lg border border-border bg-surface-strong">
        <table className="min-w-full divide-y divide-border">
          <thead className="bg-surface">
            <tr>
              <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-subtle">
                Name
              </th>
              <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-subtle">
                Players
              </th>
              <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-subtle">
                Status
              </th>
              {showActions && (
                <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-subtle">
                  {actionsLabel}
                </th>
              )}
            </tr>
          </thead>
          <tbody className="divide-y divide-border bg-surface-strong">
            {filteredGames.length === 0 ? (
              <tr>
                <td
                  colSpan={showActions ? 4 : 3}
                  className="px-4 py-8 text-center text-subtle"
                >
                  No games match your search
                </td>
              </tr>
            ) : (
              filteredGames.map((game) => (
                <tr
                  key={game.id}
                  className="transition-colors hover:bg-surface"
                >
                  <td className="px-4 py-3 whitespace-nowrap">
                    <div className="text-sm font-medium text-foreground">
                      {game.name}
                    </div>
                    <div className="text-xs text-subtle">ID: {game.id}</div>
                  </td>
                  <td className="px-4 py-3 whitespace-nowrap">
                    <span className="text-sm text-foreground">
                      {game.player_count} / {game.max_players}
                    </span>
                  </td>
                  <td className="px-4 py-3 whitespace-nowrap">
                    <span
                      className={`inline-flex px-2 py-1 text-xs font-semibold rounded-full ${
                        game.state === 'LOBBY'
                          ? 'bg-success/15 text-success-foreground'
                          : game.state === 'COMPLETED'
                            ? 'bg-muted/10 text-subtle'
                            : 'bg-warning/15 text-warning-foreground'
                      }`}
                    >
                      {game.state}
                    </span>
                  </td>
                  {showActions && (
                    <td className="px-4 py-3 whitespace-nowrap">
                      {(() => {
                        const content = renderActions?.(game)
                        return (
                          content ?? (
                            <span className="text-sm text-muted">—</span>
                          )
                        )
                      })()}
                    </td>
                  )}
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  )
}
