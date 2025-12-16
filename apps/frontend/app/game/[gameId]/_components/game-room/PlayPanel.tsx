'use client'

import { type FormEvent } from 'react'
import type { Card, TrickSnapshot } from '@/lib/game-room/types'
import { getPlayerDisplayName } from '@/utils/player-names'
import type { GameRoomViewProps } from '../game-room-view'
import { PlayingCard } from './PlayingCard'

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
  const activeName = getPlayerDisplayName(
    phase.to_act,
    play.viewerSeat,
    playerNames
  )
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
    <section className="flex w-full flex-col gap-4 rounded-3xl border border-primary/40 bg-primary/15 p-5 text-foreground shadow-[0_30px_90px_rgba(239,149,74,0.25)]">
      <header className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <h2 className="text-sm font-semibold uppercase tracking-[0.4em]">
            Play card
          </h2>
          <p className="text-xs text-muted">
            Choose a legal card from your hand. Only allowed cards stay enabled.
          </p>
        </div>
        <div className="rounded-full border border-primary/50 bg-primary/25 px-3 py-1 text-xs font-semibold text-foreground">
          Waiting on: {activeName}
        </div>
      </header>

      <form
        className="flex flex-col gap-4 rounded-2xl border border-primary/30 bg-surface/85 p-4 shadow-inner shadow-primary/20"
        onSubmit={handleSubmit}
      >
        <div className="flex flex-col items-start gap-3 text-sm text-foreground">
          <span className="text-xs uppercase tracking-wide text-muted">
            Selected card
          </span>
          {selectedCard ? (
            <PlayingCard card={selectedCard} size="md" />
          ) : (
            <span className="rounded-xl border border-primary/40 bg-background px-4 py-2 text-base font-semibold text-foreground">
              —
            </span>
          )}
        </div>
        <button
          type="submit"
          className="w-full rounded-2xl bg-primary px-4 py-3 text-base font-semibold text-primary-foreground shadow-lg shadow-primary/40 transition hover:bg-primary/90 disabled:cursor-not-allowed disabled:bg-primary/40 disabled:text-primary-foreground/70"
          disabled={isSubmitDisabled}
          aria-label={
            play.isPending
              ? 'Playing card'
              : isViewerTurn && selectedCard
                ? `Play selected card: ${selectedCard}`
                : isViewerTurn
                  ? 'Play selected card'
                  : `Waiting for ${activeName} to play`
          }
        >
          {play.isPending ? (
            'Playing…'
          ) : isViewerTurn ? (
            <>
              <span className="sm:hidden">Play card</span>
              <span className="hidden sm:inline">Play selected card</span>
            </>
          ) : (
            `Waiting for ${activeName}`
          )}
        </button>
        <p className="text-xs text-muted">
          <span className="sm:hidden">Legal:</span>
          <span className="hidden sm:inline">Legal cards:</span>{' '}
          {play.playable.length ? play.playable.join(', ') : '—'}
        </p>
      </form>
    </section>
  )
}
