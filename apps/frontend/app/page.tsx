import { auth, signIn } from '@/auth'
import { redirect } from 'next/navigation'

export default async function Home() {
  const session = await auth()

  // If authenticated, redirect to lobby
  if (session) {
    redirect('/lobby')
  }

  const appName = process.env.NEXT_PUBLIC_APP_NAME || 'Nommie'

  return (
    <main className="flex min-h-screen items-center justify-center bg-background">
      <div className="text-center">
        <h1 className="mb-4 text-6xl font-bold text-foreground">
          🃏 {appName}
        </h1>
        <p className="mb-8 text-xl text-muted">
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
            className="rounded-lg bg-primary px-6 py-3 text-lg font-medium text-primary-foreground transition-colors hover:bg-primary/90"
          >
            Sign in with Google
          </button>
        </form>
      </div>
    </main>
  )
}
