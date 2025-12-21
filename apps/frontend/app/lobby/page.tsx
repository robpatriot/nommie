import { auth } from '@/auth'
import { redirect } from 'next/navigation'
import { getTranslations } from 'next-intl/server'
import LobbyClient from '@/components/LobbyClient'
import ErrorBoundaryWithTranslations from '@/components/ErrorBoundaryWithTranslations'
import { getAvailableGames } from '@/lib/api'
import { BreadcrumbSetter } from '@/components/header-breadcrumbs'
import { handleAllowlistError } from '@/lib/auth/allowlist'

export default async function LobbyPage() {
  const t = await getTranslations('lobby')
  const session = await auth()

  // Protect route: redirect to home if not authenticated
  if (!session) {
    redirect('/')
  }

  // Fetch games, but handle backend startup gracefully
  // Error handling is centralized in fetchWithAuth - errors during startup
  // are suppressed automatically. Start with empty games list on error.
  // If user is not allowed (403 EMAIL_NOT_ALLOWED), sign them out and
  // redirect to an access denied page via handleAllowlistError.
  let allGames: Awaited<ReturnType<typeof getAvailableGames>> = []
  try {
    allGames = await getAvailableGames()
  } catch (error) {
    await handleAllowlistError(error)
    // Silently handle other errors - centralized error handling in fetchWithAuth
    // will log appropriately based on startup window and backend status.
    // The client component can handle refresh once backend is available.
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

  const creatorName = session.user?.name || t('you')

  return (
    <ErrorBoundaryWithTranslations>
      <BreadcrumbSetter crumbs={[{ label: t('breadcrumbs.lobby') }]} />
      <LobbyClient
        joinableGames={joinableGames}
        inProgressGames={inProgressGames}
        creatorName={creatorName}
      />
    </ErrorBoundaryWithTranslations>
  )
}
