import { useTranslations } from 'next-intl'
import type { Card, Seat } from '@/lib/game-room/types'
import { LastTrickCards } from './LastTrickCards'

interface LastTrickProps {
  lastTrick: Array<[Seat, Card]> | null
  getSeatName: (seat: Seat) => string
  viewerSeat: Seat
}

export function LastTrick({
  lastTrick,
  getSeatName,
  viewerSeat,
}: LastTrickProps) {
  const t = useTranslations('game.gameRoom')

  if (!lastTrick || lastTrick.length === 0) {
    return (
      <section className="flex w-full flex-col gap-4 rounded-3xl border border-white/10 bg-surface/80 p-5 text-sm text-muted shadow-[0_25px_80px_rgba(0,0,0,0.35)] backdrop-blur">
        <header className="flex items-center justify-between">
          <h2 className="text-sm font-semibold uppercase tracking-[0.4em] text-subtle">
            {t('lastTrick.title')}
          </h2>
        </header>
        <p className="text-xs text-muted">{t('lastTrick.empty')}</p>
      </section>
    )
  }

  return (
    <section className="flex w-full flex-col gap-4 rounded-3xl border border-white/10 bg-surface/80 p-5 text-sm text-muted shadow-[0_25px_80px_rgba(0,0,0,0.35)] backdrop-blur">
      <header className="flex items-center justify-between">
        <h2 className="text-sm font-semibold uppercase tracking-[0.4em] text-subtle">
          {t('lastTrick.title')}
        </h2>
      </header>
      <LastTrickCards
        lastTrick={lastTrick}
        getSeatName={getSeatName}
        viewerSeat={viewerSeat}
      />
    </section>
  )
}
