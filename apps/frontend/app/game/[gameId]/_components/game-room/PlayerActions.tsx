import { useTranslations } from 'next-intl'
import type { Card, PhaseSnapshot, Seat } from '@/lib/game-room/types'
import { BiddingPanel } from './BiddingPanel'
import { LastTrick } from './LastTrick'
import { TrumpSelectPanel } from './TrumpSelectPanel'
import type { GameRoomViewProps } from '../game-room-view'

interface PlayerActionsProps {
  phase: PhaseSnapshot
  viewerSeat: Seat
  playerNames: [string, string, string, string]
  bidding?: GameRoomViewProps['biddingState']
  trump?: GameRoomViewProps['trumpState']
  lastTrick?: Array<[Seat, Card]> | null
  seatDisplayName: (seat: Seat) => string
}

export function PlayerActions({
  phase,
  viewerSeat,
  playerNames,
  bidding,
  trump,
  lastTrick,
  seatDisplayName,
}: PlayerActionsProps) {
  const t = useTranslations('game.gameRoom.tableActions')

  if (phase.phase === 'Bidding' && bidding) {
    return (
      <BiddingPanel
        phase={phase.data}
        viewerSeat={bidding.viewerSeat}
        layoutSeat={viewerSeat}
        playerNames={playerNames}
        bidding={bidding}
      />
    )
  }

  if (phase.phase === 'TrumpSelect') {
    return (
      <TrumpSelectPanel
        phase={phase.data}
        viewerSeat={viewerSeat}
        playerNames={playerNames}
        trump={trump}
      />
    )
  }

  if (phase.phase === 'Trick') {
    return (
      <LastTrick
        lastTrick={lastTrick ?? null}
        getSeatName={seatDisplayName}
        viewerSeat={viewerSeat}
      />
    )
  }

  return (
    <section className="flex w-full flex-col gap-3 rounded-3xl border border-white/10 bg-surface/80 p-5 text-sm text-muted shadow-[0_25px_80px_rgba(0,0,0,0.35)] backdrop-blur">
      <header className="flex items-center justify-between">
        <h2 className="text-sm font-semibold uppercase tracking-[0.4em] text-subtle">
          {t('title')}
        </h2>
        <span className="rounded-full bg-surface px-3 py-1 text-xs text-muted">
          {t('waitingBadge')}
        </span>
      </header>
      <p>{t('description')}</p>
    </section>
  )
}
