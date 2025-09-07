'use client'

interface AuthControlProps {
  state: 'unauthenticated' | 'authenticated' | 'loading'
  onLogin?: () => void
  onLogout?: () => void
}

export default function AuthControl({
  state,
  onLogin,
  onLogout,
}: AuthControlProps) {
  if (state === 'loading') {
    return (
      <div role="status" aria-label="Authentication status">
        <button disabled aria-label="Loading authentication">
          Loading...
        </button>
      </div>
    )
  }

  if (state === 'authenticated') {
    return (
      <div role="status" aria-label="Authentication status">
        <button onClick={onLogout} aria-label="Sign out">
          Sign Out
        </button>
      </div>
    )
  }

  // unauthenticated state
  return (
    <div role="status" aria-label="Authentication status">
      <button onClick={onLogin} aria-label="Sign in">
        Sign In
      </button>
    </div>
  )
}
