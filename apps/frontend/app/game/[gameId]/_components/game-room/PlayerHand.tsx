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
      <section className="mx-auto flex w-full max-w-4xl flex-col gap-4 rounded-3xl border border-white/10 bg-surface/85 p-5 shadow-[0_25px_80px_rgba(0,0,0,0.35)] backdrop-blur">
        <header className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
          <h2 className="text-sm font-semibold uppercase tracking-[0.4em] text-subtle">
            Your hand
          </h2>
          <span
            className="rounded-full bg-surface px-3 py-1 text-xs text-muted"
            aria-live="polite"
          >
            {handStatus}
          </span>
        </header>
        <div className="flex gap-3 overflow-x-auto pb-2 pt-1 sm:flex-wrap sm:justify-center [scrollbar-width:none] [-ms-overflow-style:none] [&::-webkit-scrollbar]:hidden">
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
                  className={`min-w-[4rem] rounded-2xl border px-4 py-3 text-2xl font-semibold tracking-wide transition sm:min-w-0 sm:text-xl ${
                    isSelected
                      ? 'border-success bg-success/20 text-foreground shadow-lg shadow-success/30'
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
