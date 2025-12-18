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
      className={`rounded bg-primary px-3 py-1 text-sm font-semibold text-primary-foreground transition-colors hover:bg-primary/90 ${className || ''}`}
    >
      {/* Arrow only - shown below 275px */}
      <span className="min-[275px]:hidden">▶</span>
      {/* "Last Game" - shown between 275px and sm (640px) */}
      <span className="hidden min-[275px]:inline sm:hidden">▶ Last Game</span>
      {/* "Most Recent Game" - shown at sm (640px) and above */}
      <span className="hidden sm:inline">▶ Most Recent Game</span>
    </button>
  )
}
