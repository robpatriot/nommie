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
    <section className="mx-auto flex w-full max-w-4xl flex-col gap-3 rounded-2xl border border-border bg-surface/60 p-4 text-sm text-muted">
      <header className="flex items-center justify-between">
        <h2 className="text-sm font-semibold uppercase tracking-wide text-subtle">
          Table Actions
        </h2>
        <span className="text-xs text-subtle">Interactive controls</span>
      </header>
      <p>
        No interactive controls are available for the current phase. They will
        appear here when required.
      </p>
    </section>
  )
}
