import { auth } from '@/auth'
import { redirect } from 'next/navigation'
import Link from 'next/link'

export default async function LobbyPage() {
  const session = await auth()

  // Protect route: redirect to home if not authenticated
  if (!session) {
    redirect('/')
  }

  return (
    <main className="min-h-screen bg-gray-50 py-12">
      <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="bg-white shadow rounded-lg p-8">
          <div className="text-center">
            <h1 className="text-3xl font-bold text-gray-900 mb-4">
              🎮 Game Lobby
            </h1>
            <p className="text-lg text-gray-600 mb-6">
              Find or create a game to start playing Nomination Whist
            </p>
            <p className="text-sm text-gray-500">
              This is a placeholder. Game listing and creation coming in Stage
              2.
            </p>
            <div className="mt-8 space-x-4">
              <Link
                href="/"
                className="inline-block bg-gray-200 hover:bg-gray-300 px-4 py-2 rounded text-gray-700"
              >
                Back to Home
              </Link>
              <Link
                href="/game/test-game-id"
                className="inline-block bg-blue-600 hover:bg-blue-700 px-4 py-2 rounded text-white"
              >
                Test Game Route
              </Link>
            </div>
          </div>
        </div>
      </div>
    </main>
  )
}
