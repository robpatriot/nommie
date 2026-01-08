import { useTranslations } from 'next-intl'
import type {
  BiddingSnapshot,
  Card,
  PhaseSnapshot,
  Seat,
} from '@/lib/game-room/types'
import { BiddingPanel } from './BiddingPanel'
import { LastTrick } from './LastTrick'
import type { GameRoomViewProps } from '../game-room-view'
import {
  isBiddingPhase,
  isTrumpSelectPhase,
  isTrickPhase,
} from './phase-helpers'

interface PlayerActionsProps {
  phase: PhaseSnapshot
  viewerSeat: Seat | null
  playerNames: [string, string, string, string]
  bidding?: GameRoomViewProps['biddingState']
  trump?: GameRoomViewProps['trumpState']
  lastTrick?: Array<[Seat, Card]> | null
  seatDisplayName: (seat: Seat) => string
}

// Create a minimal bidding state fallback for viewing bids when no active bidding state exists
// For spectators (viewerSeat === null), use seat 0 as a placeholder for layout purposes only
function createBiddingStateFallback(
  viewerSeat: Seat | null
): NonNullable<GameRoomViewProps['biddingState']> {
  return {
    viewerSeat: viewerSeat ?? 0, // Use 0 as placeholder for spectators (layout only)
    isPending: false,
    onSubmit: async () => {},
    zeroBidLocked: false,
  }
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

  if (isBiddingPhase(phase)) {
    // Always show bidding panel during bidding phase, even if user has submitted
    // Create a minimal bidding state if one doesn't exist (for viewing bids after submission)
    const biddingState = bidding ?? createBiddingStateFallback(viewerSeat)
    return (
      <BiddingPanel
        phase={phase.data}
        viewerSeat={viewerSeat}
        layoutSeat={viewerSeat ?? 0}
        playerNames={playerNames}
        bidding={biddingState}
      />
    )
  }

  if (isTrumpSelectPhase(phase)) {
    // Show bids panel during trump selection (with or without trump selection UI)
    // If it's your turn, show trump selection UI integrated into bidding panel
    // If not your turn, show just the bids panel with indication
    // Always show, even if user has already submitted their bid
    const biddingState = bidding ?? createBiddingStateFallback(viewerSeat)
    // Construct a BiddingSnapshot from TrumpSelectSnapshot data
    // This allows BiddingPanel to display bids during trump selection phase
    const biddingSnapshot: BiddingSnapshot = {
      round: phase.data.round,
      to_act: phase.data.to_act,
      bids: phase.data.round.bids,
      min_bid: 0,
      max_bid: phase.data.round.hand_size,
      last_trick: phase.data.last_trick,
      previous_round: null,
    }
    return (
      <BiddingPanel
        phase={biddingSnapshot}
        viewerSeat={viewerSeat}
        layoutSeat={viewerSeat ?? 0}
        playerNames={playerNames}
        bidding={biddingState}
        trumpPhase={phase.data}
        trump={trump}
      />
    )
  }

  if (isTrickPhase(phase)) {
    return (
      <LastTrick
        lastTrick={lastTrick ?? null}
        getSeatName={seatDisplayName}
        viewerSeat={viewerSeat ?? 0}
      />
    )
  }

  return (
    <section className="flex w-full flex-col gap-3 rounded-3xl border border-border/60 bg-card/80 p-5 text-sm text-muted-foreground shadow-elevated backdrop-blur">
      <header className="flex items-center justify-between">
        <h2 className="text-sm font-semibold uppercase tracking-[0.4em] text-muted-foreground">
          {t('title')}
        </h2>
        <span className="rounded-full bg-card px-3 py-1 text-xs text-muted-foreground">
          {t('waitingBadge')}
        </span>
      </header>
      <p>{t('description')}</p>
    </section>
  )
}
