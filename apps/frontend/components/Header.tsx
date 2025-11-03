import { auth, signIn, signOut } from '@/auth'
import Link from 'next/link'
import ResumeGameButton from './ResumeGameButton'

export default async function Header() {
  const session = await auth()

  return (
    <header className="w-full flex items-center justify-between gap-3 p-4 bg-white border-b border-gray-200">
      <div className="flex items-center gap-4">
        <Link href="/" className="text-xl font-bold text-gray-900">
          🃏 Nommie
        </Link>
        {session?.user && (
          <Link
            href="/lobby"
            className="text-sm text-gray-700 hover:text-gray-900 hover:underline"
          >
            Lobby
          </Link>
        )}
      </div>
      <div className="flex items-center gap-3">
        {session?.user ? (
          <>
            <ResumeGameButton />
            <span className="text-sm text-gray-600">{session.user.email}</span>
            <form
              action={async () => {
                'use server'
                await signOut()
              }}
            >
              <button
                type="submit"
                className="bg-gray-200 hover:bg-gray-300 px-3 py-1 rounded text-sm"
              >
                Sign out
              </button>
            </form>
          </>
        ) : (
          <form
            action={async () => {
              'use server'
              await signIn('google')
            }}
          >
            <button
              type="submit"
              className="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded text-sm"
            >
              Sign in with Google
            </button>
          </form>
        )}
      </div>
    </header>
  )
}
