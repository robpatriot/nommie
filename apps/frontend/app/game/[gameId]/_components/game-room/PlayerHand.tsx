'use client'

import { useMemo, useRef, useEffect, useState } from 'react'
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
  onPlayCard?: (card: Card) => Promise<void> | void
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
  onPlayCard,
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
  } else if (isTrickPhase && !viewerTurn) {
    handStatus = `Waiting for ${waitingOnName ?? 'next player'}`
  }

  const containerRef = useRef<HTMLDivElement>(null)
  const [overlapAmount, setOverlapAmount] = useState(16) // Default -ml-4 (16px)

  useEffect(() => {
    if (!containerRef.current || viewerHand.length === 0) {
      if (viewerHand.length === 0) {
        setOverlapAmount(16) // Reset to default when empty
      }
      return
    }

    const updateOverlap = () => {
      const container = containerRef.current
      if (!container) return

      // Measure container width (accounting for padding)
      const containerWidth = container.clientWidth

      // Card width: md size is w-20 (80px) + padding p-[2px] (4px total) = ~84px
      // We'll measure the first card button to get exact width
      const firstCard = container.querySelector('button')
      if (!firstCard) {
        // If no card found yet, retry after a short delay
        return
      }

      const cardWidth = firstCard.offsetWidth
      const cardCount = viewerHand.length

      if (cardCount <= 1) {
        setOverlapAmount(0)
        return
      }

      // Calculate how much space we need
      const totalWidthNeeded = cardWidth * cardCount
      const availableWidth = containerWidth

      // If cards already fit, use minimal overlap
      if (totalWidthNeeded <= availableWidth) {
        setOverlapAmount(16) // Default overlap
        return
      }

      // Calculate overlap needed: we want the last card to fit
      // Total visible width = cardWidth + (cardCount - 1) * (cardWidth - overlap)
      // We want: cardWidth + (cardCount - 1) * (cardWidth - overlap) <= availableWidth
      // Solving for overlap: overlap >= (totalWidthNeeded - availableWidth) / (cardCount - 1)
      const overlapNeeded =
        (totalWidthNeeded - availableWidth) / (cardCount - 1)

      // Ensure overlap is at least a minimum (so cards are still distinct)
      // and at most cardWidth - some visible portion (we want to see at least rank/suit)
      const minOverlap = 4
      const maxOverlap = cardWidth - 40 // Ensure at least 40px of each card is visible

      const newOverlap = Math.max(
        minOverlap,
        Math.min(maxOverlap, overlapNeeded)
      )
      setOverlapAmount(newOverlap)
    }

    // Use multiple timing strategies to ensure we measure after DOM updates
    // Strategy 1: Double RAF (catches immediate DOM updates)
    const rafId1 = requestAnimationFrame(() => {
      requestAnimationFrame(updateOverlap)
    })

    // Strategy 2: setTimeout fallback (catches delayed React updates)
    const timeoutId = setTimeout(() => {
      updateOverlap()
    }, 150)

    // Use ResizeObserver to handle container size changes
    const resizeObserver = new ResizeObserver(() => {
      requestAnimationFrame(() => {
        requestAnimationFrame(updateOverlap)
      })
    })
    resizeObserver.observe(containerRef.current)

    // Also handle window resize
    const handleResize = () => {
      requestAnimationFrame(() => {
        requestAnimationFrame(updateOverlap)
      })
    }
    window.addEventListener('resize', handleResize)

    return () => {
      cancelAnimationFrame(rafId1)
      clearTimeout(timeoutId)
      resizeObserver.disconnect()
      window.removeEventListener('resize', handleResize)
    }
  }, [viewerHand.length])

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

  const sidePadding = Math.max(24, overlapAmount + 12)

  return (
    <section
      className={cn(
        'flex w-full flex-col gap-3 rounded-[28px] border border-white/15 bg-surface/80 p-4 text-foreground shadow-[0_35px_80px_rgba(0,0,0,0.4)] backdrop-blur',
        className
      )}
    >
      <header className="grid grid-cols-3 items-center gap-3">
        <div className="flex flex-col gap-1">
          <span className="text-[11px] font-semibold uppercase tracking-[0.4em] text-subtle">
            Your hand
          </span>
          {handStatus !== 'Read-only preview' && (
            <p className="text-xs text-muted" aria-live="polite">
              {handStatus}
            </p>
          )}
        </div>
        {isTrickPhase && playState ? (
          <div className="flex justify-center">
            <button
              type="button"
              onClick={async () => {
                if (onPlayCard && selectedCard && viewerTurn) {
                  await onPlayCard(selectedCard)
                  onSelectCard(null)
                }
              }}
              disabled={
                !viewerTurn ||
                playState.isPending ||
                !selectedCard ||
                !playState.playable.includes(selectedCard)
              }
              className="rounded-2xl bg-primary px-4 py-1.5 text-sm font-semibold text-primary-foreground shadow-lg shadow-primary/40 transition hover:bg-primary/90 disabled:cursor-not-allowed disabled:bg-primary/40 disabled:text-primary-foreground/70"
              aria-label={
                playState.isPending
                  ? 'Playing card'
                  : selectedCard
                    ? `Play selected card: ${selectedCard}`
                    : 'Select a card to play'
              }
            >
              {playState.isPending
                ? 'Playing…'
                : viewerTurn
                  ? 'Play selected card'
                  : `Waiting for ${waitingOnName ?? 'next player'}`}
            </button>
          </div>
        ) : (
          <div />
        )}
        {isTrickPhase && playState ? (
          <div className="flex items-center justify-end gap-2">
            {playState.playable.length > 0 ? (
              <div className="rounded-lg border border-white/15 bg-surface/60 px-2 py-1">
                <span className="text-xs font-medium text-muted">
                  Legal cards:
                </span>
                <span className="ml-1.5 text-xs font-medium text-foreground">
                  {playState.playable.length === viewerHand.length &&
                  viewerHand.every((card) => playState.playable.includes(card))
                    ? 'all'
                    : playState.playable.join(', ')}
                </span>
              </div>
            ) : null}
            {!viewerTurn ? (
              <span className="rounded-full border border-white/15 bg-surface px-3 py-1 text-xs font-semibold text-subtle">
                Waiting on {waitingOnName ?? '—'}
              </span>
            ) : null}
          </div>
        ) : (
          <div />
        )}
      </header>
      <div
        ref={containerRef}
        className="flex justify-center overflow-x-visible overflow-y-visible pb-2 pt-4"
        style={{
          paddingLeft: sidePadding,
          paddingRight: sidePadding,
        }}
      >
        {viewerHand.length === 0 ? (
          <span className="text-sm text-subtle">
            Hand will appear once available.
          </span>
        ) : (
          viewerHand.map((card, index) => {
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
                style={{
                  marginLeft: index === 0 ? 0 : -overlapAmount,
                  transitionProperty: 'margin-left, transform, scale, z-index',
                  transitionDuration: '300ms',
                  transitionTimingFunction: 'ease-out',
                }}
                className={cn(
                  'relative focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/60 disabled:cursor-not-allowed',
                  'hover:z-10 hover:translate-y-[-4px] hover:scale-105',
                  isSelected
                    ? 'z-20 translate-y-[-8px] scale-110'
                    : isPlayable && viewerTurn
                      ? 'z-0'
                      : ''
                )}
                aria-label={cardLabel}
                aria-pressed={isSelected}
              >
                <div
                  className={cn(
                    'rounded-[1.45rem] border-2 p-[2px] transition-all',
                    isSelected ? 'border-primary' : 'border-transparent'
                  )}
                >
                  <PlayingCard
                    card={card}
                    size="md"
                    isDimmed={isDisabled && !isSelected}
                    isSelected={isSelected}
                  />
                </div>
              </button>
            )
          })
        )}
      </div>
    </section>
  )
}
