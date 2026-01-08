'use client'

import { useTranslations } from 'next-intl'
import { useRouter } from 'next/navigation'
import { Button } from '@/components/ui/Button'

export function GameJoinErrorClient({
  code,
  status,
}: {
  code?: string
  status?: number
}) {
  const router = useRouter()
  const t = useTranslations('game.gameRoom')

  let title = t('joinError.genericTitle')
  let message = t('joinError.genericMessage')

  if (code === 'GAME_NOT_FOUND' || status === 404) {
    title = t('joinError.notFoundTitle')
    message = t('joinError.notFoundMessage')
  } else if (code === 'PHASE_MISMATCH') {
    title = t('joinError.phaseTitle')
    message = t('joinError.phaseMessage')
  } else if (code === 'VALIDATION_ERROR') {
    title = t('joinError.fullTitle')
    message = t('joinError.fullMessage')
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/80">
      <div className="w-full max-w-sm rounded-2xl bg-card p-6 shadow-elevated">
        <h2 className="text-lg font-semibold text-foreground">{title}</h2>
        <p className="mt-2 text-sm text-muted-foreground">{message}</p>
        <div className="mt-4 flex justify-end">
          <Button
            type="button"
            variant="primary"
            size="md"
            className="rounded-full"
            onClick={() => {
              router.push('/lobby')
            }}
          >
            {t('joinError.okButton')}
          </Button>
        </div>
      </div>
    </div>
  )
}
