'use client'

import { useState, useEffect } from 'react'
import { useRouter } from 'next/navigation'
import { getLastActiveGame, BackendApiError } from '@/lib/api'

interface ResumeGameButtonProps {
  className?: string
}

export default function ResumeGameButton({ className }: ResumeGameButtonProps) {
  const router = useRouter()
  const [lastActiveGameId, setLastActiveGameId] = useState<number | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    const loadLastActive = async () => {
      try {
        const gameId = await getLastActiveGame()
        setLastActiveGameId(gameId)
      } catch (err) {
        // Silently fail - endpoint might not exist yet
        if (err instanceof BackendApiError && err.status === 404) {
          // Expected - endpoint not implemented yet
        } else {
          console.error('Error loading last active game:', err)
        }
      } finally {
        setLoading(false)
      }
    }

    loadLastActive()
  }, [])

  if (loading || !lastActiveGameId) {
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
