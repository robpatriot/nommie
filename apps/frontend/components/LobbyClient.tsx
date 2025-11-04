'use client'

import { useState } from 'react'
import { useRouter } from 'next/navigation'
import GameList from './GameList'
import CreateGameModal from './CreateGameModal'
import Toast, { type ToastMessage } from './Toast'
import { createGameAction, joinGameAction } from '@/app/actions/game-actions'
import { BackendApiError } from '@/lib/api'
import type { Game } from '@/lib/types'

type LobbyClientProps = {
  joinableGames: Game[]
  inProgressGames: Game[]
  lastActiveGameId: number | null
  creatorName: string
}

export default function LobbyClient({
  joinableGames: initialJoinable,
  inProgressGames: initialInProgress,
  lastActiveGameId,
  creatorName,
}: LobbyClientProps) {
  const router = useRouter()
  const [joinableGames] = useState<Game[]>(initialJoinable)
  const [inProgressGames] = useState<Game[]>(initialInProgress)
  const [refreshing, setRefreshing] = useState(false)
  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false)
  const [toast, setToast] = useState<ToastMessage | null>(null)

  const handleRefresh = async () => {
    setRefreshing(true)
    router.refresh()
    // Reset refreshing state after a short delay
    setTimeout(() => setRefreshing(false), 500)
  }

  const showToast = (
    message: string,
    type: 'success' | 'error',
    error?: BackendApiError
  ) => {
    setToast({
      id: Date.now().toString(),
      message,
      type,
      error,
    })
  }

  const handleCreateGame = async (
    name: string,
    startingDealerPos: number | null
  ) => {
    const result = await createGameAction({
      name,
      starting_dealer_pos: startingDealerPos,
    })

    if (result.error) {
      showToast(
        result.error.message || 'Failed to create game',
        'error',
        result.error
      )
      // Log traceId in dev
      if (process.env.NODE_ENV === 'development' && result.error.traceId) {
        console.error('Create game error traceId:', result.error.traceId)
      }
      throw result.error // Re-throw so modal can handle it
    }

    showToast('Game created successfully!', 'success')
    // Refresh the page to show the new game
    router.refresh()
    // Navigate to the new game
    router.push(`/game/${result.game.id}`)
  }

  const handleJoin = async (gameId: number) => {
    const result = await joinGameAction(gameId)

    if (result.error) {
      showToast(
        result.error.message || 'Failed to join game',
        'error',
        result.error
      )
      // Log traceId in dev
      if (process.env.NODE_ENV === 'development' && result.error.traceId) {
        console.error('Join game error traceId:', result.error.traceId)
      }
      return
    }

    showToast('Joined game successfully!', 'success')
    router.push(`/game/${gameId}`)
  }

  const handleResume = () => {
    if (lastActiveGameId) {
      router.push(`/game/${lastActiveGameId}`)
    }
  }

  return (
    <>
      <div className="min-h-screen bg-gray-50 py-12">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="mb-6">
            <div className="bg-white shadow rounded-lg p-6">
              <div className="flex items-center justify-between mb-4">
                <h1 className="text-3xl font-bold text-gray-900">
                  🎮 Game Lobby
                </h1>
                <div className="flex items-center gap-3">
                  <button
                    onClick={() => setIsCreateModalOpen(true)}
                    className="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded text-sm font-medium transition-colors"
                  >
                    Create Game
                  </button>
                  <button
                    onClick={handleRefresh}
                    disabled={refreshing}
                    className="bg-gray-200 hover:bg-gray-300 px-4 py-2 rounded text-sm font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    {refreshing ? 'Refreshing...' : 'Refresh'}
                  </button>
                </div>
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

      <CreateGameModal
        isOpen={isCreateModalOpen}
        onClose={() => setIsCreateModalOpen(false)}
        onCreateGame={handleCreateGame}
        creatorName={creatorName}
      />

      <Toast toast={toast} onClose={() => setToast(null)} />
    </>
  )
}
