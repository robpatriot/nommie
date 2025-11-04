import { auth } from '@/auth'
import { redirect } from 'next/navigation'
import LobbyClient from '@/components/LobbyClient'
import {
  getJoinableGames,
  getInProgressGames,
  getLastActiveGame,
} from '@/lib/api'

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
    <LobbyClient
      joinableGames={joinableGames}
      inProgressGames={inProgressGames}
      lastActiveGameId={lastActiveGameId}
      creatorName={creatorName}
    />
  )
}
