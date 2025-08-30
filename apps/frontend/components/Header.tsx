import { auth, signIn, signOut } from '@/auth'

export default async function Header() {
  const session = await auth()

  return (
    <header className="w-full flex items-center justify-end gap-3 p-4">
      {session?.user ? (
        <>
          <span className="text-sm text-gray-600">{session.user.email}</span>
          <form
            action={async () => {
              'use server'
              await signOut()
            }}
          >
            <button
              type="submit"
              className="bg-gray-200 hover:bg-gray-300 px-3 py-1 rounded"
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
            className="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded"
          >
            Sign in with Google
          </button>
        </form>
      )}
    </header>
  )
}
