import { auth } from '@/auth'
import { redirect } from 'next/navigation'
import LobbyClient from '@/components/LobbyClient'

export default async function LobbyPage() {
  const session = await auth()

  // Protect route: redirect to home if not authenticated
  if (!session) {
    redirect('/')
  }

  return <LobbyClient />
}
