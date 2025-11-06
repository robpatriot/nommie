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

  // [AUTH_BYPASS] START - Temporary debugging feature - remove when done
  const disableAuth = process.env.NEXT_PUBLIC_DISABLE_AUTH === 'true'
  // Protect route: redirect to home if not authenticated (unless bypass enabled)
  if (!session && !disableAuth) {
    redirect('/')
  }
  // [AUTH_BYPASS] END
  const [joinableGames, inProgressGames, lastActiveGameId] = await Promise.all([
    getJoinableGames(),
    getInProgressGames(),
    getLastActiveGame(),
  ])

  // [AUTH_BYPASS] - Handle null session when auth disabled
  const creatorName = session?.user?.name || 'You'

  return (
    <LobbyClient
      joinableGames={joinableGames}
      inProgressGames={inProgressGames}
      lastActiveGameId={lastActiveGameId}
      creatorName={creatorName}
    />
  )
}
