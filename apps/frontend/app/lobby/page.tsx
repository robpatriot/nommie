import { auth } from '@/auth'
import { redirect } from 'next/navigation'
import LobbyClient from '@/components/LobbyClient'
import ErrorBoundary from '@/components/ErrorBoundary'
import {
  getJoinableGames,
  getInProgressGames,
  getLastActiveGame,
} from '@/lib/api'
import { BreadcrumbSetter } from '@/components/header-breadcrumbs'

export default async function LobbyPage() {
  const session = await auth()

  // Protect route: redirect to home if not authenticated
  if (!session) {
    redirect('/')
  }

  const [joinableGames, inProgressGames, lastActiveGameId] = await Promise.all([
    getJoinableGames(),
    getInProgressGames(),
    getLastActiveGame(),
  ])

  const creatorName = session.user?.name || 'You'

  return (
    <ErrorBoundary>
      <BreadcrumbSetter crumbs={[{ label: 'Lobby' }]} />
      <LobbyClient
        joinableGames={joinableGames}
        inProgressGames={inProgressGames}
        lastActiveGameId={lastActiveGameId}
        creatorName={creatorName}
      />
    </ErrorBoundary>
  )
}
