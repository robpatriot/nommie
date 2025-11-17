import type { Card, PhaseSnapshot, Seat } from '@/lib/game-room/types'
import { BiddingPanel } from './BiddingPanel'
import { PlayPanel } from './PlayPanel'
import { TrumpSelectPanel } from './TrumpSelectPanel'
import type { GameRoomViewProps } from '../game-room-view'

interface PlayerActionsProps {
  phase: PhaseSnapshot
  viewerSeat: Seat
  playerNames: [string, string, string, string]
  bidding?: GameRoomViewProps['biddingState']
  trump?: GameRoomViewProps['trumpState']
  play?: GameRoomViewProps['playState']
  selectedCard: Card | null
  onPlayCard: (card: Card) => Promise<void> | void
}

export function PlayerActions({
  phase,
  viewerSeat,
  playerNames,
  bidding,
  trump,
  play,
  selectedCard,
  onPlayCard,
}: PlayerActionsProps) {
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

  if (phase.phase === 'Trick' && play) {
    return (
      <PlayPanel
        phase={phase.data}
        playerNames={playerNames}
        play={play}
        selectedCard={selectedCard}
        onPlayCard={onPlayCard}
      />
    )
  }

  return (
    <section className="mx-auto flex w-full max-w-4xl flex-col gap-3 rounded-3xl border border-white/10 bg-surface/80 p-5 text-sm text-muted shadow-[0_25px_80px_rgba(0,0,0,0.35)] backdrop-blur">
      <header className="flex items-center justify-between">
        <h2 className="text-sm font-semibold uppercase tracking-[0.4em] text-subtle">
          Table actions
        </h2>
        <span className="rounded-full bg-surface px-3 py-1 text-xs text-muted">
          Waiting for next phase
        </span>
      </header>
      <p>
        Interactive controls will surface here as soon as the current phase
        requires your input.
      </p>
    </section>
  )
}
