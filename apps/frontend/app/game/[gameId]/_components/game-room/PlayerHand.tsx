'use client'

import { useMemo } from 'react'
import type { Card, PhaseSnapshot, Seat } from '@/lib/game-room/types'
import type { GameRoomViewProps } from '../game-room-view'
import { cn } from '@/lib/cn'
import { PlayingCard } from './PlayingCard'

interface PlayerHandProps {
  viewerHand: Card[]
  phase: PhaseSnapshot
  playerNames: [string, string, string, string]
  viewerSeat: Seat
  playState?: GameRoomViewProps['playState']
  selectedCard: Card | null
  onSelectCard: (card: Card | null) => void
  className?: string
}

export function PlayerHand({
  viewerHand,
  phase,
  playerNames,
  viewerSeat,
  playState,
  selectedCard,
  onSelectCard,
  className,
}: PlayerHandProps) {
  const isTrickPhase = phase.phase === 'Trick' && !!playState
  const viewerTurn =
    isTrickPhase &&
    playState &&
    phase.phase === 'Trick' &&
    phase.data.to_act === playState.viewerSeat
  const playableCards = useMemo(
    () => new Set(playState?.playable ?? []),
    [playState]
  )
  const waitingOnSeat = phase.phase === 'Trick' ? phase.data.to_act : null
  const waitingOnName =
    waitingOnSeat === null
      ? null
      : waitingOnSeat === viewerSeat
        ? 'You'
        : playerNames[waitingOnSeat]

  let handStatus = 'Read-only preview'

  if (!viewerHand.length) {
    handStatus = 'Hand will appear once the game starts.'
  } else if (isTrickPhase) {
    if (!viewerTurn) {
      handStatus = `Waiting for ${waitingOnName ?? 'next player'} to play`
    } else if (playState?.isPending) {
      handStatus = 'Playing card…'
    } else if (selectedCard) {
      handStatus = `Selected ${selectedCard}`
    } else {
      handStatus = 'Select a card to play'
    }
  }

  const handleCardClick = (card: Card) => {
    if (!isTrickPhase || !playState) {
      return
    }

    const isPlayable = playableCards.has(card)
    if (!viewerTurn || !isPlayable || playState.isPending) {
      return
    }

    onSelectCard(selectedCard === card ? null : card)
  }

  return (
    <section
      className={cn(
        'flex w-full flex-col gap-3 rounded-[28px] border border-white/15 bg-surface/80 p-4 text-foreground shadow-[0_35px_80px_rgba(0,0,0,0.4)] backdrop-blur',
        className
      )}
    >
      <header className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-col gap-1">
          <span className="text-[11px] font-semibold uppercase tracking-[0.4em] text-subtle">
            Your hand
          </span>
          <p className="text-xs text-muted" aria-live="polite">
            {handStatus}
          </p>
        </div>
        {isTrickPhase ? (
          <span className="rounded-full border border-white/15 bg-surface px-3 py-1 text-xs font-semibold text-subtle">
            {viewerTurn ? 'Your play' : `Waiting on ${waitingOnName ?? '—'}`}
          </span>
        ) : null}
      </header>
      <div className="flex gap-2 overflow-x-auto pb-2 pt-1 sm:justify-center sm:gap-3 [scrollbar-width:none] [-ms-overflow-style:none] [&::-webkit-scrollbar]:hidden">
        {viewerHand.length === 0 ? (
          <span className="text-sm text-subtle">
            Hand will appear once available.
          </span>
        ) : (
          viewerHand.map((card) => {
            const isPlayable = playableCards.has(card)
            const isSelected = selectedCard === card
            const isDisabled =
              !isTrickPhase ||
              !playState ||
              !isPlayable ||
              !viewerTurn ||
              playState.isPending

            const cardLabel = isPlayable
              ? `${card}, ${isSelected ? 'selected' : 'playable'}`
              : `${card}, ${isDisabled ? 'not playable' : 'playable'}`
            return (
              <button
                key={card}
                type="button"
                onClick={() => handleCardClick(card)}
                disabled={isDisabled}
                className={cn(
                  'relative rounded-[1.45rem] border border-transparent p-[2px] transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/60 disabled:cursor-not-allowed',
                  isSelected
                    ? 'border-success/80 bg-success/20 shadow-[0_20px_35px_rgba(34,197,94,0.35)]'
                    : isPlayable && viewerTurn
                      ? 'hover:-translate-y-1 hover:bg-primary/10'
                      : 'opacity-60'
                )}
                aria-label={cardLabel}
                aria-pressed={isSelected}
              >
                <PlayingCard
                  card={card}
                  size="md"
                  isDimmed={isDisabled && !isSelected}
                  isSelected={isSelected}
                />
              </button>
            )
          })
        )}
      </div>
    </section>
  )
}
