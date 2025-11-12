'use client'

import { type FormEvent } from 'react'
import type { Card, TrickSnapshot } from '@/lib/game-room/types'
import type { GameRoomViewProps } from '../game-room-view'

interface PlayPanelProps {
  phase: TrickSnapshot
  playerNames: [string, string, string, string]
  play: NonNullable<GameRoomViewProps['playState']>
  selectedCard: Card | null
  onPlayCard: (card: Card) => Promise<void> | void
}

export function PlayPanel({
  phase,
  playerNames,
  play,
  selectedCard,
  onPlayCard,
}: PlayPanelProps) {
  const isViewerTurn = phase.to_act === play.viewerSeat
  const activeName =
    phase.to_act === play.viewerSeat ? 'You' : playerNames[phase.to_act]
  const isCardPlayable = !!selectedCard && play.playable.includes(selectedCard)
  const isSubmitDisabled = !isViewerTurn || play.isPending || !isCardPlayable

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()

    if (isSubmitDisabled || !selectedCard) {
      return
    }

    await onPlayCard(selectedCard)
  }

  return (
    <section className="mx-auto flex w-full max-w-4xl flex-col gap-4 rounded-2xl border border-primary/40 bg-primary/10 p-4">
      <header className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <h2 className="text-sm font-semibold uppercase tracking-wide text-primary-foreground">
            Play Card
          </h2>
          <p className="text-xs text-primary-foreground/80">
            Choose a legal card from your hand. Only legal cards are enabled.
          </p>
        </div>
        <div className="rounded-full border border-primary/40 bg-primary/15 px-3 py-1 text-xs font-medium text-primary-foreground">
          Waiting on: {activeName}
        </div>
      </header>

      <form
        className="flex flex-col gap-3 rounded-lg border border-primary/30 bg-surface/60 p-4 shadow-inner shadow-primary/20"
        onSubmit={handleSubmit}
      >
        <div className="flex flex-wrap items-center gap-3 text-sm text-primary-foreground">
          <span className="text-xs uppercase tracking-wide text-primary-foreground/80">
            Selected Card
          </span>
          <span className="rounded-md border border-primary/40 bg-background px-3 py-1 font-semibold text-foreground">
            {selectedCard ?? '—'}
          </span>
        </div>
        <button
          type="submit"
          className="w-full rounded-md bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground transition hover:bg-primary/80 disabled:cursor-not-allowed disabled:bg-primary/40 disabled:text-primary-foreground/70"
          disabled={isSubmitDisabled}
        >
          {play.isPending
            ? 'Playing…'
            : isViewerTurn
              ? 'Play Selected Card'
              : `Waiting for ${activeName}`}
        </button>
        <p className="text-xs text-primary-foreground/80">
          Legal cards: {play.playable.length ? play.playable.join(', ') : '—'}
        </p>
      </form>
    </section>
  )
}
