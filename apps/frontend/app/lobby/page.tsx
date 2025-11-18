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

  const allGames = await getAvailableGames()

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
