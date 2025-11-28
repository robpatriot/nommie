import { auth } from '@/auth'
import { redirect } from 'next/navigation'
import LobbyClient from '@/components/LobbyClient'
import ErrorBoundary from '@/components/ErrorBoundary'
import { getAvailableGames } from '@/lib/api'
import { BreadcrumbSetter } from '@/components/header-breadcrumbs'

export default async function LobbyPage() {
  const session = await auth()

  // Protect route: redirect to home if not authenticated
  if (!session) {
    redirect('/')
  }

  // Fetch games, but handle backend startup gracefully
  let allGames: Awaited<ReturnType<typeof getAvailableGames>> = []
  try {
    allGames = await getAvailableGames()
  } catch (error) {
    // If backend is not ready yet (connection error), start with empty games list
    // The client component can handle refresh once backend is available
    const errorMessage =
      error instanceof Error ? error.message.toLowerCase() : ''
    // Access cause property safely (may not exist in all TypeScript lib versions)
    const causeMessage =
      error instanceof Error && 'cause' in error && error.cause instanceof Error
        ? error.cause.message.toLowerCase()
        : ''

    const isConnectionError =
      error instanceof Error &&
      (errorMessage.includes('econnrefused') ||
        errorMessage.includes('fetch failed') ||
        errorMessage.includes('authentication required') ||
        causeMessage.includes('econnrefused') ||
        causeMessage.includes('connect econnrefused'))

    // Only log non-connection errors (connection/auth errors during startup are expected)
    if (!isConnectionError) {
      console.error('Failed to fetch games', error)
    }
    // allGames remains empty array - client will show empty state and can retry
  }

  const lobbyGames = allGames.filter((game) => game.state === 'LOBBY')
  const joinableGames = lobbyGames.filter(
    (game) => game.player_count < game.max_players
  )
  const fullLobbyGames = lobbyGames.filter(
    (game) => game.player_count >= game.max_players
  )
  const activeGames = allGames.filter((game) => game.state !== 'LOBBY')

  const inProgressGamesMap = new Map<number, (typeof activeGames)[number]>()
  for (const game of [...fullLobbyGames, ...activeGames]) {
    inProgressGamesMap.set(game.id, game)
  }
  const inProgressGames = Array.from(inProgressGamesMap.values())

  const creatorName = session.user?.name || 'You'

  return (
    <ErrorBoundary>
      <BreadcrumbSetter crumbs={[{ label: 'Lobby' }]} />
      <LobbyClient
        joinableGames={joinableGames}
        inProgressGames={inProgressGames}
        creatorName={creatorName}
      />
    </ErrorBoundary>
  )
}
