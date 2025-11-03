import { auth } from '@/auth'
import { redirect } from 'next/navigation'
import Link from 'next/link'

interface GamePageProps {
  params: Promise<{
    gameId: string
  }>
}

export default async function GamePage({ params }: GamePageProps) {
  const session = await auth()

  // Protect route: redirect to home if not authenticated
  if (!session) {
    redirect('/')
  }

  const { gameId } = await params

  return (
    <main className="min-h-screen bg-gray-50 py-12">
      <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="bg-white shadow rounded-lg p-8">
          <div className="text-center">
            <h1 className="text-3xl font-bold text-gray-900 mb-4">
              🎴 Game Room
            </h1>
            <p className="text-lg text-gray-600 mb-2">
              Game ID:{' '}
              <code className="font-mono text-sm bg-gray-100 px-2 py-1 rounded">
                {gameId}
              </code>
            </p>
            <p className="text-sm text-gray-500 mb-6">
              This is a placeholder. Game view and interactions coming in Stage
              4-5.
            </p>
            <div className="mt-8 space-x-4">
              <Link
                href="/lobby"
                className="inline-block bg-gray-200 hover:bg-gray-300 px-4 py-2 rounded text-gray-700"
              >
                Back to Lobby
              </Link>
            </div>
          </div>
        </div>
      </div>
    </main>
  )
}
