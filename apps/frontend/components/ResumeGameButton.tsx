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
      {/* "Last Game" - shown between 275px and sm (640px) */}
      <span className="hidden min-[275px]:inline sm:hidden">
        {t('lastGame')}
      </span>
      {/* "Most Recent Game" - shown at sm (640px) and above */}
      <span className="hidden sm:inline">{t('mostRecentGame')}</span>
    </button>
  )
}
