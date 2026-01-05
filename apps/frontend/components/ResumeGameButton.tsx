'use client'

import { useRouter } from 'next/navigation'
import { useTranslations } from 'next-intl'
import { cn } from '@/lib/cn'

interface ResumeGameButtonProps {
  className?: string
  lastActiveGameId: number | null
}

export default function ResumeGameButton({
  className,
  lastActiveGameId,
}: ResumeGameButtonProps) {
  const router = useRouter()
  const t = useTranslations('common.home.resumeGame')

  if (!lastActiveGameId) {
    return null
  }

  return (
    <button
      onClick={() => router.push(`/game/${lastActiveGameId}`)}
      className={cn(
        'rounded bg-primary px-3 py-1 text-sm font-semibold text-primary-foreground transition-colors hover:bg-primary/90',
        className
      )}
    >
      {/* Arrow only - shown below 275px */}
      <span className="min-[275px]:hidden">▶</span>
      {/* "Next Game" - shown at 275px and above */}
      <span className="hidden min-[275px]:inline">{t('nextGame')}</span>
    </button>
  )
}
