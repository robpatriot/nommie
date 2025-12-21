'use client'

import { useMemo, useState } from 'react'
import { useRouter } from 'next/navigation'
import { useTranslations } from 'next-intl'
import GameList from './GameList'
import CreateGameModal from './CreateGameModal'
import Toast from './Toast'
import { PageHero } from './PageHero'
import { PageContainer } from './PageContainer'
import { StatCard } from './StatCard'
import { useToast } from '@/hooks/useToast'
import { useAvailableGames } from '@/hooks/queries/useGames'
import {
  useCreateGame,
  useJoinGame,
  useDeleteGame,
} from '@/hooks/mutations/useGameMutations'
import type { Game } from '@/lib/types'
import { toQueryError } from '@/lib/queries/query-error-handler'
import { logBackendError } from '@/lib/logging/error-logger'

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
  const t = useTranslations('lobby')
  const router = useRouter()
  const { toasts, showToast, hideToast } = useToast()
  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false)

  // Use query hook with initial data from server
  const initialAllGames = [...initialJoinable, ...initialInProgress]
  const {
    data: allGames = initialAllGames,
    refetch: refetchGames,
    isFetching: isRefreshing,
  } = useAvailableGames(initialAllGames)

  // Mutations
  const createGameMutation = useCreateGame()
  const joinGameMutation = useJoinGame()
  const deleteGameMutation = useDeleteGame()

  // Split games into joinable and in-progress
  const joinableGames = useMemo(() => {
    return allGames.filter(
      (game) => game.state === 'LOBBY' && game.player_count < game.max_players
    )
  }, [allGames])

  const inProgressGames = useMemo(() => {
    return allGames.filter(
      (game) => game.state !== 'LOBBY' || game.player_count >= game.max_players
    )
  }, [allGames])

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
    try {
      await refetchGames()
    } catch (error) {
      const message =
        error instanceof Error ? error.message : t('toasts.refreshFailed')
      showToast(message, 'error')
    }
  }

  const handleCreateGame = async (name: string) => {
    // Use default name if provided name is empty
    const defaultName = t('createGame.defaultName', { name: creatorName })
    const gameName = name.trim() || defaultName

    try {
      const game = await createGameMutation.mutateAsync({ name: gameName })
      showToast(t('toasts.createdSuccess'), 'success')
      router.push(`/game/${game.id}`)
    } catch (error) {
      const backendError = toQueryError(error, t('toasts.createFailed'))
      showToast(backendError.message, 'error', backendError)
      logBackendError('Create game failed', backendError, {
        action: 'createGame',
      })
      throw error // Re-throw so modal can handle it
    }
  }

  const handleJoin = async (gameId: number) => {
    try {
      await joinGameMutation.mutateAsync(gameId)
      showToast(t('toasts.joinedSuccess'), 'success')
      router.push(`/game/${gameId}`)
    } catch (error) {
      const backendError = toQueryError(error, t('toasts.joinFailed'))

      // Customize error message for validation errors
      let message = backendError.message
      if (
        backendError.status === 400 &&
        backendError.code === 'VALIDATION_ERROR'
      ) {
        message = t('toasts.gameFilledUp')
        router.refresh()
      }

      showToast(message, 'error', backendError)
      logBackendError('Join game failed', backendError, {
        action: 'joinGame',
        gameId,
      })
    }
  }

  const handleRejoin = (gameId: number) => {
    router.push(`/game/${gameId}`)
  }

  const handleDelete = async (gameId: number) => {
    if (!confirm(t('confirmations.deleteGame'))) {
      return
    }

    try {
      // deleteGameAction will fetch the lock_version automatically if not provided
      await deleteGameMutation.mutateAsync({ gameId })
      showToast(t('toasts.deletedSuccess'), 'success')
      router.refresh()
    } catch (error) {
      const backendError = toQueryError(error, t('toasts.deleteFailed'))

      // Customize error messages for specific status codes
      let message = backendError.message
      if (backendError.status === 428) {
        message = t('toasts.deleteMissingVersion')
      } else if (backendError.status === 409) {
        message = t('toasts.deleteConflict')
      }

      showToast(message, 'error', backendError)
      logBackendError('Delete game failed', backendError, {
        action: 'deleteGame',
        gameId,
      })
    }
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
                {t('hero.kicker')}
              </p>
              <div>
                <h1 className="text-3xl font-semibold tracking-tight text-foreground sm:text-4xl">
                  {t('hero.title')}
                </h1>
                <p className="mt-2 text-sm text-muted sm:text-base">
                  {t('hero.description')}
                </p>
              </div>
            </div>
          }
          aside={
            <div className="grid gap-3 sm:grid-cols-3">
              <StatCard
                label={t('stats.joinableTables.label')}
                value={filteredJoinableGames.length}
                description={t('stats.joinableTables.description')}
              />
              <StatCard
                label={t('stats.seatsAvailable.label')}
                value={openSeatCount}
                description={t('stats.seatsAvailable.description')}
              />
              <StatCard
                label={t('stats.inProgress.label')}
                value={sortedInProgressGames.length}
                description={t('stats.inProgress.description')}
              />
            </div>
          }
          footer={
            <div className="flex flex-col gap-3 text-sm sm:flex-row">
              <button
                onClick={() => setIsCreateModalOpen(true)}
                className="inline-flex w-full items-center justify-center rounded-2xl bg-primary px-5 py-3 font-semibold text-primary-foreground shadow-lg shadow-primary/30 transition hover:bg-primary/90"
                aria-label={t('actions.create.ariaLabel')}
              >
                <span role="img" aria-hidden>
                  ➕
                </span>
                <span className="ml-2">{t('actions.create.label')}</span>
              </button>
              <button
                onClick={handleRefresh}
                disabled={isRefreshing}
                className="inline-flex w-full items-center justify-center rounded-2xl border border-border/60 bg-surface px-5 py-3 font-semibold text-muted transition hover:border-primary/50 hover:text-foreground disabled:cursor-not-allowed disabled:opacity-50"
                aria-live="polite"
              >
                {isRefreshing
                  ? t('actions.refresh.refreshing')
                  : t('actions.refresh.label')}
              </button>
            </div>
          }
        />

        <div className="grid gap-6 lg:grid-cols-2">
          <GameList
            games={filteredJoinableGames}
            title={t('lists.joinable.title')}
            emptyMessage={t('lists.joinable.empty')}
            actionsLabel={t('lists.actionsLabel')}
            renderActions={(game) => {
              const actions = []

              if (game.viewer_is_host) {
                actions.push(
                  <button
                    key="delete"
                    onClick={() => handleDelete(game.id)}
                    className="rounded-full border border-danger/60 px-3 py-1 text-xs font-semibold uppercase tracking-wide text-danger transition hover:bg-danger hover:text-danger-foreground"
                  >
                    {t('actions.delete')}
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
                    {t('actions.goToGame')}
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
                    {t('actions.join')}
                  </button>
                )
              } else if (game.player_count >= game.max_players) {
                actions.push(
                  <span
                    key="full"
                    className="rounded-full bg-surface px-3 py-1 text-xs font-semibold uppercase tracking-wide text-subtle"
                  >
                    {t('labels.fullTable')}
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
            title={t('lists.inProgress.title')}
            emptyMessage={t('lists.inProgress.empty')}
            actionsLabel={t('lists.actionsLabel')}
            renderActions={(game) => {
              const actions = []

              if (game.viewer_is_host) {
                actions.push(
                  <button
                    key="delete"
                    onClick={() => handleDelete(game.id)}
                    className="rounded-full border border-danger/60 px-3 py-1 text-xs font-semibold uppercase tracking-wide text-danger transition hover:bg-danger hover:text-danger-foreground"
                  >
                    {t('actions.delete')}
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
                    {t('actions.rejoin')}
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
