'use client'

import { useMemo, useState } from 'react'
import { useRouter } from 'next/navigation'
import GameList from './GameList'
import CreateGameModal from './CreateGameModal'
import Toast from './Toast'
import { PageHero } from './PageHero'
import { PageContainer } from './PageContainer'
import { StatCard } from './StatCard'
import { useToast } from '@/hooks/useToast'
import { useApiAction } from '@/hooks/useApiAction'
import { BackendApiError } from '@/lib/errors'
import {
  createGameAction,
  deleteGameAction,
  joinGameAction,
} from '@/app/actions/game-actions'
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
  creatorName: string
}

export default function LobbyClient({
  joinableGames: initialJoinable,
  inProgressGames: initialInProgress,
  creatorName,
}: LobbyClientProps) {
  const router = useRouter()
  const { toasts, showToast, hideToast } = useToast()
  const [refreshing, setRefreshing] = useState(false)
  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false)

  const executeApiAction = useApiAction({
    showToast,
  })

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
    const defaultName = `${creatorName}'s game`
    const gameName = name.trim() || defaultName

    const result = await createGameAction({ name: gameName })

    if (result.kind === 'error') {
      const error = new BackendApiError(
        result.message || 'Failed to create game',
        result.status,
        result.code,
        result.traceId
      )
      showToast(result.message || 'Failed to create game', 'error', error)
      if (process.env.NODE_ENV === 'development' && result.traceId) {
        console.error('Create game error traceId:', result.traceId)
      }
      throw error // Re-throw so modal can handle it
    }

    showToast('Game created successfully!', 'success')
    router.refresh()
    router.push(`/game/${result.data.id}`)
  }

  const handleJoin = async (gameId: number) => {
    await executeApiAction(
      async () => {
        const result = await joinGameAction(gameId)
        if (result.kind === 'error') {
          // Customize error message for validation errors
          if (result.status === 400 && result.code === 'VALIDATION_ERROR') {
            router.refresh()
            return {
              kind: 'error' as const,
              message: 'That game just filled up. Please choose another one.',
              status: result.status,
              code: result.code,
              traceId: result.traceId,
            }
          }
        }
        return result
      },
      {
        successMessage: 'Joined game successfully!',
        errorMessage: 'Failed to join game',
        onSuccess: () => {
          router.push(`/game/${gameId}`)
        },
      }
    )
  }

  const handleRejoin = (gameId: number) => {
    router.push(`/game/${gameId}`)
  }

  const handleDelete = async (gameId: number) => {
    if (
      !confirm(
        'Are you sure you want to delete this game? This action cannot be undone.'
      )
    ) {
      return
    }

    // deleteGameAction will fetch the ETag automatically if not provided
    await executeApiAction(
      async () => {
        const result = await deleteGameAction(gameId)
        if (result.kind === 'error') {
          // Customize error messages for specific status codes
          let message = result.message || 'Failed to delete game'
          if (result.status === 428) {
            message =
              'Cannot delete game: missing version information. Please try again.'
          } else if (result.status === 409) {
            message =
              'Cannot delete game: game was modified. Please refresh and try again.'
          }
          return {
            kind: 'error' as const,
            message,
            status: result.status,
            code: result.code,
            traceId: result.traceId,
          }
        }
        return result
      },
      {
        successMessage: 'Game deleted successfully',
        errorMessage: 'Failed to delete game',
        onSuccess: () => {
          router.refresh()
        },
      }
    )
  }

  const openSeatCount = filteredJoinableGames.reduce((total, game) => {
    return total + Math.max(game.max_players - game.player_count, 0)
  }, 0)

  return (
    <>
      <PageContainer>
        <PageHero
          className="border-white/15"
          intro={
            <div className="space-y-4">
              <p className="text-xs font-semibold uppercase tracking-[0.4em] text-subtle">
                Game Lobby
              </p>
              <div>
                <h1 className="text-3xl font-semibold tracking-tight text-foreground sm:text-4xl">
                  Seat your table and deal the next hand.
                </h1>
                <p className="mt-2 text-sm text-muted sm:text-base">
                  Select a table, claim a seat, and begin the bidding once
                  everyone is ready.
                </p>
              </div>
            </div>
          }
          aside={
            <div className="grid gap-3 sm:grid-cols-3">
              <StatCard
                label="Joinable tables"
                value={filteredJoinableGames.length}
                description="Open public tables"
              />
              <StatCard
                label="Seats available"
                value={openSeatCount}
                description="Across joinable tables"
              />
              <StatCard
                label="In progress"
                value={sortedInProgressGames.length}
                description="Active or full tables"
              />
            </div>
          }
          footer={
            <div className="flex flex-col gap-3 text-sm sm:flex-row">
              <button
                onClick={() => setIsCreateModalOpen(true)}
                className="inline-flex w-full items-center justify-center rounded-2xl bg-primary px-5 py-3 font-semibold text-primary-foreground shadow-lg shadow-primary/30 transition hover:bg-primary/90"
                aria-label="Create a new game"
              >
                <span role="img" aria-hidden>
                  ➕
                </span>
                <span className="ml-2">Create game</span>
              </button>
              <button
                onClick={handleRefresh}
                disabled={refreshing}
                className="inline-flex w-full items-center justify-center rounded-2xl border border-border/60 bg-surface px-5 py-3 font-semibold text-muted transition hover:border-primary/50 hover:text-foreground disabled:cursor-not-allowed disabled:opacity-50"
                aria-live="polite"
              >
                {refreshing ? 'Refreshing…' : 'Refresh list'}
              </button>
            </div>
          }
        />

        <div className="grid gap-6 lg:grid-cols-2">
          <GameList
            games={filteredJoinableGames}
            title="Joinable games"
            emptyMessage="No games available to join. Create one to get started!"
            actionsLabel="Actions"
            renderActions={(game) => {
              const actions = []

              if (game.viewer_is_host) {
                actions.push(
                  <button
                    key="delete"
                    onClick={() => handleDelete(game.id)}
                    className="rounded-full border border-danger/60 px-3 py-1 text-xs font-semibold uppercase tracking-wide text-danger transition hover:bg-danger hover:text-danger-foreground"
                  >
                    Delete
                  </button>
                )
              }

              if (game.viewer_is_member) {
                actions.push(
                  <button
                    key="rejoin"
                    onClick={() => handleRejoin(game.id)}
                    className="rounded-full bg-primary/90 px-4 py-2 text-sm font-semibold text-primary-foreground shadow shadow-primary/30 transition hover:bg-primary"
                  >
                    Go to game
                  </button>
                )
              } else if (
                game.state === 'LOBBY' &&
                game.player_count < game.max_players
              ) {
                actions.push(
                  <button
                    key="join"
                    onClick={() => handleJoin(game.id)}
                    className="rounded-full bg-accent/90 px-4 py-2 text-sm font-semibold text-accent-foreground shadow shadow-accent/30 transition hover:bg-accent"
                  >
                    Join
                  </button>
                )
              } else if (game.player_count >= game.max_players) {
                actions.push(
                  <span
                    key="full"
                    className="rounded-full bg-surface px-3 py-1 text-xs font-semibold uppercase tracking-wide text-subtle"
                  >
                    Full table
                  </span>
                )
              }

              return actions.length > 0 ? (
                <div className="flex flex-wrap gap-2">{actions}</div>
              ) : null
            }}
          />

          <GameList
            games={sortedInProgressGames}
            title="In progress"
            emptyMessage="No games currently in progress."
            actionsLabel="Actions"
            renderActions={(game) => {
              const actions = []

              if (game.viewer_is_host) {
                actions.push(
                  <button
                    key="delete"
                    onClick={() => handleDelete(game.id)}
                    className="rounded-full border border-danger/60 px-3 py-1 text-xs font-semibold uppercase tracking-wide text-danger transition hover:bg-danger hover:text-danger-foreground"
                  >
                    Delete
                  </button>
                )
              }

              if (game.viewer_is_member) {
                actions.push(
                  <button
                    key="rejoin"
                    onClick={() => handleRejoin(game.id)}
                    className="rounded-full bg-primary/90 px-4 py-2 text-sm font-semibold text-primary-foreground shadow shadow-primary/30 transition hover:bg-primary"
                  >
                    Rejoin
                  </button>
                )
              }

              return actions.length > 0 ? (
                <div className="flex flex-wrap gap-2">{actions}</div>
              ) : null
            }}
          />
        </div>
      </PageContainer>

      <CreateGameModal
        isOpen={isCreateModalOpen}
        onClose={() => setIsCreateModalOpen(false)}
        onCreateGame={handleCreateGame}
        creatorName={creatorName}
      />

      <Toast toasts={toasts} onClose={hideToast} />
    </>
  )
}
