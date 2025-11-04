'use client'

import { useRouter } from 'next/navigation'

interface ResumeGameButtonProps {
  className?: string
  lastActiveGameId: number | null
}

export default function ResumeGameButton({
  className,
  lastActiveGameId,
}: ResumeGameButtonProps) {
  const router = useRouter()

  if (!lastActiveGameId) {
    return null
  }

  return (
    <button
      onClick={() => router.push(`/game/${lastActiveGameId}`)}
      className={`text-sm bg-blue-600 hover:bg-blue-700 text-white px-3 py-1 rounded transition-colors ${className || ''}`}
    >
      ▶ Resume Game
    </button>
  )
}
