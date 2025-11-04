'use client'

import { useState } from 'react'
import { useRouter } from 'next/navigation'
import GameList from './GameList'
import type { Game } from '@/lib/types'

type LobbyClientProps = {
  joinableGames: Game[]
  inProgressGames: Game[]
  lastActiveGameId: number | null
}

export default function LobbyClient({
  joinableGames: initialJoinable,
  inProgressGames: initialInProgress,
  lastActiveGameId,
}: LobbyClientProps) {
  const router = useRouter()
  const [joinableGames] = useState<Game[]>(initialJoinable)
  const [inProgressGames] = useState<Game[]>(initialInProgress)
  const [refreshing, setRefreshing] = useState(false)

  const handleRefresh = () => {
    setRefreshing(true)
    router.refresh()
  }

  const handleJoin = (gameId: number) => {
    router.push(`/game/${gameId}`)
  }

  const handleResume = () => {
    if (lastActiveGameId) {
      router.push(`/game/${lastActiveGameId}`)
    }
  }

  return (
    <div className="min-h-screen bg-gray-50 py-12">
      <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="mb-6">
          <div className="bg-white shadow rounded-lg p-6">
            <div className="flex items-center justify-between mb-4">
              <h1 className="text-3xl font-bold text-gray-900">
                🎮 Game Lobby
              </h1>
              <button
                onClick={handleRefresh}
                disabled={refreshing}
                className="bg-gray-200 hover:bg-gray-300 px-4 py-2 rounded text-sm font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {refreshing ? 'Refreshing...' : 'Refresh'}
              </button>
            </div>

            {lastActiveGameId && (
              <div className="mb-4">
                <button
                  onClick={handleResume}
                  className="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded text-sm font-medium transition-colors"
                >
                  ▶ Resume Last Game
                </button>
              </div>
            )}
          </div>
        </div>

        <div className="space-y-6">
          <GameList
            games={joinableGames}
            title="Joinable Games"
            emptyMessage="No games available to join. Create one to get started!"
            onJoin={handleJoin}
            showJoinButton={true}
          />

          <GameList
            games={inProgressGames}
            title="In Progress (View Only)"
            emptyMessage="No games currently in progress."
            showJoinButton={false}
          />
        </div>
      </div>
    </div>
  )
}
