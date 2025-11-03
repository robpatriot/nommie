'use client'

import { useState, useEffect } from 'react'
import { useRouter } from 'next/navigation'
import GameList from './GameList'
import type { Game } from '@/lib/types'
import {
  getJoinableGames,
  getInProgressGames,
  getLastActiveGame,
  BackendApiError,
} from '@/lib/api'

export default function LobbyClient() {
  const router = useRouter()
  const [joinableGames, setJoinableGames] = useState<Game[]>([])
  const [inProgressGames, setInProgressGames] = useState<Game[]>([])
  const [lastActiveGameId, setLastActiveGameId] = useState<number | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [refreshing, setRefreshing] = useState(false)

  const loadGames = async () => {
    try {
      setError(null)
      const [joinable, inProgress, lastActive] = await Promise.all([
        getJoinableGames(),
        getInProgressGames(),
        getLastActiveGame(),
      ])

      setJoinableGames(joinable)
      setInProgressGames(inProgress)
      setLastActiveGameId(lastActive)
    } catch (err) {
      const message =
        err instanceof BackendApiError
          ? `Error loading games: ${err.message}`
          : 'Failed to load games'
      setError(message)
      console.error('Error loading games:', err)
    } finally {
      setLoading(false)
      setRefreshing(false)
    }
  }

  useEffect(() => {
    loadGames()
  }, [])

  const handleRefresh = () => {
    setRefreshing(true)
    loadGames()
  }

  const handleJoin = (gameId: number) => {
    router.push(`/game/${gameId}`)
  }

  const handleResume = () => {
    if (lastActiveGameId) {
      router.push(`/game/${lastActiveGameId}`)
    }
  }

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-50 py-12">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="bg-white shadow rounded-lg p-8">
            <div className="animate-pulse">
              <div className="h-8 bg-gray-200 rounded w-1/3 mb-4"></div>
              <div className="h-4 bg-gray-200 rounded w-2/3 mb-8"></div>
              <div className="space-y-3">
                <div className="h-4 bg-gray-200 rounded"></div>
                <div className="h-4 bg-gray-200 rounded"></div>
                <div className="h-4 bg-gray-200 rounded"></div>
              </div>
            </div>
          </div>
        </div>
      </div>
    )
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

            {error && (
              <div className="mb-4 bg-red-50 border border-red-200 rounded-md p-4">
                <p className="text-sm text-red-800">{error}</p>
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
