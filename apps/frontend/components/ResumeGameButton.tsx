'use client'

import { useRouter } from 'next/navigation'
import { useTranslations } from 'next-intl'
import { cn } from '@/lib/cn'

interface ResumeGameButtonProps {
  className?: string
  waitingGameId: number | null
}

export default function ResumeGameButton({
  className,
  waitingGameId,
}: ResumeGameButtonProps) {
  const router = useRouter()
  const t = useTranslations('common.home.resumeGame')

  if (!waitingGameId) {
    return null
  }

  return (
    <button
      onClick={() => router.push(`/game/${waitingGameId}`)}
      className={cn(
        'rounded bg-primary px-3 py-1 text-sm font-semibold text-primary-foreground transition-colors hover:bg-primary/90',
        className
      )}
    >
      {/* Arrow only - shown below 275px */}
      <span className="min-[275px]:hidden">▶</span>
      {/* "▶ Next" - shown at 275px to 349px */}
      <span className="hidden min-[275px]:inline min-[350px]:hidden">
        {t('next')}
      </span>
      {/* "▶ Next Game" - shown at 350px and above */}
      <span className="hidden min-[350px]:inline">{t('nextGame')}</span>
    </button>
  )
}
