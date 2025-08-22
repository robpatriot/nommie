export default function Home() {
  const appName = process.env.NEXT_PUBLIC_APP_NAME || 'Nommie'

  return (
    <main className="min-h-screen flex items-center justify-center bg-gray-50">
      <div className="text-center">
        <h1 className="text-6xl font-bold text-gray-900 mb-4">🃏 {appName}</h1>
        <p className="text-xl text-gray-600">
          Welcome to the multiplayer Nomination Whist game!
        </p>
        <p className="text-sm text-gray-500 mt-4">
          Frontend running on Next.js with App Router
        </p>
      </div>
    </main>
  )
}
