'use client'

import { useState, useMemo } from 'react'
import type { Game } from '@/lib/types'

interface GameListProps {
  games: Game[]
  title: string
  emptyMessage: string
  onJoin?: (gameId: number) => void
  showJoinButton?: boolean
}

export default function GameList({
  games,
  title,
  emptyMessage,
  onJoin,
  showJoinButton = true,
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
        <h2 className="text-xl font-semibold text-gray-900 mb-4">{title}</h2>
        <div className="bg-gray-50 border border-gray-200 rounded-lg p-6 text-center">
          <p className="text-gray-600">{emptyMessage}</p>
        </div>
      </div>
    )
  }

  return (
    <div className="mb-8">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-semibold text-gray-900">{title}</h2>
        <input
          type="text"
          placeholder="Search games..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="px-3 py-1.5 border border-gray-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>

      <div className="bg-white border border-gray-200 rounded-lg overflow-hidden">
        <table className="min-w-full divide-y divide-gray-200">
          <thead className="bg-gray-50">
            <tr>
              <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                Name
              </th>
              <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                Players
              </th>
              <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                Status
              </th>
              {showJoinButton && (
                <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Actions
                </th>
              )}
            </tr>
          </thead>
          <tbody className="bg-white divide-y divide-gray-200">
            {filteredGames.length === 0 ? (
              <tr>
                <td
                  colSpan={showJoinButton ? 4 : 3}
                  className="px-4 py-8 text-center text-gray-500"
                >
                  No games match your search
                </td>
              </tr>
            ) : (
              filteredGames.map((game) => (
                <tr
                  key={game.id}
                  className="hover:bg-gray-50 transition-colors"
                >
                  <td className="px-4 py-3 whitespace-nowrap">
                    <div className="text-sm font-medium text-gray-900">
                      {game.name}
                    </div>
                    <div className="text-xs text-gray-500">ID: {game.id}</div>
                  </td>
                  <td className="px-4 py-3 whitespace-nowrap">
                    <span className="text-sm text-gray-900">
                      {game.player_count} / {game.max_players}
                    </span>
                  </td>
                  <td className="px-4 py-3 whitespace-nowrap">
                    <span
                      className={`inline-flex px-2 py-1 text-xs font-semibold rounded-full ${
                        game.state === 'LOBBY'
                          ? 'bg-green-100 text-green-800'
                          : game.state === 'COMPLETED'
                            ? 'bg-gray-100 text-gray-800'
                            : 'bg-yellow-100 text-yellow-800'
                      }`}
                    >
                      {game.state}
                    </span>
                  </td>
                  {showJoinButton && (
                    <td className="px-4 py-3 whitespace-nowrap">
                      {game.state === 'LOBBY' &&
                      game.player_count < game.max_players ? (
                        <button
                          onClick={() => onJoin?.(game.id)}
                          className="text-sm bg-blue-600 hover:bg-blue-700 text-white px-3 py-1 rounded transition-colors"
                        >
                          Join
                        </button>
                      ) : (
                        <span className="text-sm text-gray-400">—</span>
                      )}
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
