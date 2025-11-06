import { auth, signIn } from '@/auth'
import { redirect } from 'next/navigation'

export default async function Home() {
  const session = await auth()

  // [AUTH_BYPASS] START - Temporary debugging feature - remove when done
  const disableAuth = process.env.NEXT_PUBLIC_DISABLE_AUTH === 'true'
  // If authenticated, redirect to lobby (unless bypass enabled - then auto-redirect)
  if (session || disableAuth) {
    redirect('/lobby')
  }
  // [AUTH_BYPASS] END

  const appName = process.env.NEXT_PUBLIC_APP_NAME || 'Nommie'

  return (
    <main className="min-h-screen flex items-center justify-center bg-gray-50">
      <div className="text-center">
        <h1 className="text-6xl font-bold text-gray-900 mb-4">🃏 {appName}</h1>
        <p className="text-xl text-gray-600 mb-8">
          Welcome to the multiplayer Nomination Whist game!
        </p>
        <form
          action={async () => {
            'use server'
            await signIn('google')
          }}
        >
          <button
            type="submit"
            className="bg-blue-600 hover:bg-blue-700 text-white px-6 py-3 rounded-lg text-lg font-medium transition-colors"
          >
            Sign in with Google
          </button>
        </form>
      </div>
    </main>
  )
}
