'use client'

import { useMemo, useState } from 'react'
import { useRouter } from 'next/navigation'
import GameList from './GameList'
import CreateGameModal from './CreateGameModal'
import Toast from './Toast'
import { useToast } from '@/hooks/useToast'
import { createGameAction, joinGameAction } from '@/app/actions/game-actions'
import type { Game } from '@/lib/types'

const sortByUpdatedAtDesc = (a: Game, b: Game) => {
  const aTime = Date.parse(a.updated_at)
  const bTime = Date.parse(b.updated_at)

  if (Number.isNaN(aTime) && Number.isNaN(bTime)) {
    return 0
  }
  if (Number.isNaN(aTime)) {
    return 1
  }
  if (Number.isNaN(bTime)) {
    return -1
  }
  return bTime - aTime
}

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
  const { toast, showToast, hideToast } = useToast()
  const [refreshing, setRefreshing] = useState(false)
  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false)

  const joinableGames = initialJoinable
  const inProgressGames = initialInProgress

  const filteredJoinableGames = useMemo(() => {
    const openGames = joinableGames.filter(
      (game) => game.state === 'LOBBY' && game.player_count < game.max_players
    )
    return openGames.slice().sort(sortByUpdatedAtDesc)
  }, [joinableGames])

  const sortedInProgressGames = useMemo(() => {
    const memberGames: Game[] = []
    const otherGames: Game[] = []

    for (const game of inProgressGames) {
      if (game.viewer_is_member) {
        memberGames.push(game)
      } else {
        otherGames.push(game)
      }
    }

    const sortedMember = memberGames.slice().sort(sortByUpdatedAtDesc)
    const sortedOthers = otherGames.slice().sort(sortByUpdatedAtDesc)

    return [...sortedMember, ...sortedOthers]
  }, [inProgressGames])

  const handleRefresh = async () => {
    setRefreshing(true)
    router.refresh()
    // Reset refreshing state after a short delay
    setTimeout(() => setRefreshing(false), 500)
  }

  const handleCreateGame = async (name: string) => {
    // Use default name if provided name is empty
    const defaultName = `${creatorName} game`
    const gameName = name.trim() || defaultName

    const result = await createGameAction({
      name: gameName,
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
      let message = result.error.message || 'Failed to join game'

      if (
        result.error.status === 400 &&
        result.error.code === 'VALIDATION_ERROR'
      ) {
        message = 'That game just filled up. Please choose another one.'
        router.refresh()
      }

      showToast(message, 'error', result.error)
      // Log traceId in dev
      if (process.env.NODE_ENV === 'development' && result.error.traceId) {
        console.error('Join game error traceId:', result.error.traceId)
      }
      return
    }

    showToast('Joined game successfully!', 'success')
    router.push(`/game/${gameId}`)
  }

  const handleRejoin = (gameId: number) => {
    router.push(`/game/${gameId}`)
  }

  const handleResume = () => {
    if (lastActiveGameId) {
      router.push(`/game/${lastActiveGameId}`)
    }
  }

  return (
    <>
      <div className="min-h-screen bg-background py-12">
        <div className="mx-auto max-w-4xl px-4 sm:px-6 lg:px-8">
          <div className="mb-6">
            <div className="rounded-lg border border-border bg-surface-strong shadow-elevated p-6">
              <div className="mb-4 flex items-center justify-between">
                <h1 className="text-3xl font-bold text-foreground">
                  🎮 Game Lobby
                </h1>
                <div className="flex items-center gap-3">
                  <button
                    onClick={() => setIsCreateModalOpen(true)}
                    className="rounded bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
                    aria-label="Create a new game"
                  >
                    Create Game
                  </button>
                  <button
                    onClick={handleRefresh}
                    disabled={refreshing}
                    className="rounded bg-surface px-4 py-2 text-sm font-medium text-muted transition-colors hover:bg-surface-strong hover:text-foreground disabled:cursor-not-allowed disabled:opacity-50"
                    aria-label={
                      refreshing ? 'Refreshing game list' : 'Refresh game list'
                    }
                  >
                    {refreshing ? 'Refreshing...' : 'Refresh'}
                  </button>
                </div>
              </div>

              {lastActiveGameId && (
                <div className="mb-4">
                  <button
                    onClick={handleResume}
                    className="rounded bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
                    aria-label="Resume most recent game"
                  >
                    ▶ Most Recent Game
                  </button>
                </div>
              )}
            </div>
          </div>

          <div className="space-y-6">
            <GameList
              games={filteredJoinableGames}
              title="Joinable Games"
              emptyMessage="No games available to join. Create one to get started!"
              actionsLabel="Actions"
              renderActions={(game) => {
                // If user is already a member, show "Go to game" button
                if (game.viewer_is_member) {
                  return (
                    <button
                      onClick={() => handleRejoin(game.id)}
                      className="rounded bg-primary px-3 py-1 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
                    >
                      Go to game
                    </button>
                  )
                }

                // If game is joinable and user is not a member, show "Join" button
                if (
                  game.state === 'LOBBY' &&
                  game.player_count < game.max_players
                ) {
                  return (
                    <button
                      onClick={() => handleJoin(game.id)}
                      className="rounded bg-primary px-3 py-1 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
                    >
                      Join
                    </button>
                  )
                }

                // Game is full
                return <span className="text-sm text-muted">Game is full</span>
              }}
            />

            <GameList
              games={sortedInProgressGames}
              title="In Progress Games"
              emptyMessage="No games currently in progress."
              actionsLabel="Resume"
              renderActions={(game) =>
                game.viewer_is_member ? (
                  <button
                    onClick={() => handleRejoin(game.id)}
                    className="rounded bg-primary px-3 py-1 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
                  >
                    Rejoin
                  </button>
                ) : null
              }
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

      <Toast toast={toast} onClose={hideToast} />
    </>
  )
}
