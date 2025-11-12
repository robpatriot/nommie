'use client'

import { useMemo } from 'react'
import type { Card, PhaseSnapshot, Seat } from '@/lib/game-room/types'
import type { GameRoomViewProps } from '../game-room-view'

interface PlayerHandProps {
  viewerHand: Card[]
  phase: PhaseSnapshot
  playerNames: [string, string, string, string]
  viewerSeat: Seat
  playState?: GameRoomViewProps['playState']
  selectedCard: Card | null
  onSelectCard: (card: Card | null) => void
}

export function PlayerHand({
  viewerHand,
  phase,
  playerNames,
  viewerSeat,
  playState,
  selectedCard,
  onSelectCard,
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
    <section className="mx-auto flex w-full max-w-4xl flex-col gap-3 rounded-2xl border border-border bg-surface/70 p-4">
      <header className="flex items-center justify-between">
        <h2 className="text-sm uppercase tracking-wide text-subtle">
          Your Hand
        </h2>
        <span className="text-xs text-muted">{handStatus}</span>
      </header>
      <div className="flex flex-wrap justify-center gap-2">
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
                className={`rounded-xl border px-3 py-2 text-lg font-semibold tracking-wide transition ${
                  isSelected
                    ? 'border-success bg-success/20 text-foreground shadow-md shadow-success/30'
                    : isPlayable && viewerTurn
                      ? 'border-success/60 bg-surface text-foreground hover:border-success hover:bg-success/10'
                      : 'border-border bg-surface text-muted'
                } ${
                  isDisabled
                    ? 'cursor-not-allowed opacity-60'
                    : 'cursor-pointer'
                }`}
                aria-label={cardLabel}
                aria-pressed={isSelected}
              >
                {card}
              </button>
            )
          })
        )}
      </div>
    </section>
  )
}
