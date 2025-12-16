'use client'

import { useLayoutEffect, useMemo, useRef, useState } from 'react'
import type { Card, PhaseSnapshot, Seat } from '@/lib/game-room/types'
import type { GameRoomViewProps } from '../game-room-view'
import { cn } from '@/lib/cn'
import { PlayingCard, CARD_DIMENSIONS } from './PlayingCard'

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
  requireCardConfirmation?: boolean
}

const CARD_WIDTH = CARD_DIMENSIONS.md.width
const CARD_HEIGHT = CARD_DIMENSIONS.md.height
const CARD_PADDING = 8 // padding + border
const EFFECTIVE_CARD_WIDTH = CARD_WIDTH + CARD_PADDING

const MIN_OVERLAP = 4
// Allow more aggressive overlap: keep only a very small portion of the top-left corner visible
const MAX_OVERLAP = CARD_WIDTH - 17
const AESTHETIC_OVERLAP = 8
const BLEND_THRESHOLD = 20 // Pixels of remaining space before we start blending

function calculateOverlap(
  viewportWidth: number,
  cardCount: number
): { overlap: number; needsScroll: boolean } {
  if (cardCount <= 1) {
    return { overlap: 0, needsScroll: false }
  }

  const totalWidthNeeded = EFFECTIVE_CARD_WIDTH * cardCount

  // Calculate minimum possible width with maximum overlap
  // This is the absolute minimum space the cards can occupy
  const minWidthWithMaxOverlap =
    EFFECTIVE_CARD_WIDTH +
    (cardCount - 1) * (EFFECTIVE_CARD_WIDTH - MAX_OVERLAP)

  // Only enable scroll if viewport is below the absolute minimum
  // This prevents scrollbar from appearing during intermediate resize states
  const needsScroll = viewportWidth < minWidthWithMaxOverlap

  const spaceRemaining = viewportWidth - totalWidthNeeded

  if (spaceRemaining >= BLEND_THRESHOLD) {
    // Plenty of space: use aesthetic overlap
    return { overlap: AESTHETIC_OVERLAP, needsScroll }
  }

  if (spaceRemaining > 0) {
    // Tight but fits: blend between aesthetic and calculated
    const overlapNeeded = (totalWidthNeeded - viewportWidth) / (cardCount - 1)
    const calculatedOverlap = Math.max(
      MIN_OVERLAP,
      Math.min(MAX_OVERLAP, overlapNeeded)
    )
    const blend = 1 - spaceRemaining / BLEND_THRESHOLD
    const overlap =
      AESTHETIC_OVERLAP + (calculatedOverlap - AESTHETIC_OVERLAP) * blend
    return { overlap, needsScroll }
  }

  // Doesn't fit: calculate required overlap
  const overlapNeeded = (totalWidthNeeded - viewportWidth) / (cardCount - 1)
  const overlap = Math.max(MIN_OVERLAP, Math.min(MAX_OVERLAP, overlapNeeded))

  return { overlap, needsScroll }
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
  requireCardConfirmation = true,
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
  } else if (viewerTurn && isTrickPhase && !requireCardConfirmation) {
    handStatus = 'Tap a legal card to play immediately.'
  }

  const viewportRef = useRef<HTMLDivElement>(null)
  const [needsScroll, setNeedsScroll] = useState(false)
  const [baseOffset, setBaseOffset] = useState(0)
  const [cardStep, setCardStep] = useState(
    EFFECTIVE_CARD_WIDTH - AESTHETIC_OVERLAP
  )
  const [minHeight, setMinHeight] = useState(CARD_HEIGHT + 16)

  useLayoutEffect(() => {
    const viewport = viewportRef.current
    if (!viewport) {
      return
    }

    const updateLayout = () => {
      const width = viewport.clientWidth
      const { overlap: newOverlap, needsScroll: newNeedsScroll } =
        calculateOverlap(width, viewerHand.length)
      setNeedsScroll(newNeedsScroll)

      const newCardStep = EFFECTIVE_CARD_WIDTH - newOverlap
      setCardStep(newCardStep)
      setMinHeight(CARD_HEIGHT + (newNeedsScroll ? 24 : 16))

      // Calculate baseOffset for centering cards when not scrolling
      if (!newNeedsScroll && viewerHand.length > 0) {
        const stripWidth =
          EFFECTIVE_CARD_WIDTH + (viewerHand.length - 1) * newCardStep
        setBaseOffset((width - stripWidth) / 2)
      } else {
        setBaseOffset(0)
      }
    }

    updateLayout()

    const resizeObserver = new ResizeObserver(() => {
      updateLayout()
    })
    resizeObserver.observe(viewport)

    return () => {
      resizeObserver.disconnect()
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

    if (!requireCardConfirmation) {
      onSelectCard(null)
      if (onPlayCard) {
        void onPlayCard(card)
      }
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
            {requireCardConfirmation ? (
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
                {playState.isPending ? (
                  'Playing…'
                ) : viewerTurn ? (
                  <>
                    <span className="sm:hidden">Play card</span>
                    <span className="hidden sm:inline">Play selected card</span>
                  </>
                ) : (
                  `Waiting for ${waitingOnName ?? 'next player'}`
                )}
              </button>
            ) : (
              <span className="rounded-full border border-white/15 bg-surface px-4 py-1 text-xs font-semibold text-subtle">
                Tap any legal card to play
              </span>
            )}
          </div>
        ) : (
          <div />
        )}
        {isTrickPhase && playState ? (
          <div className="flex items-center justify-end gap-2">
            {playState.playable.length > 0 ? (
              <div className="rounded-lg border border-white/15 bg-surface/60 px-2 py-1">
                <span className="text-xs font-medium text-muted">
                  <span className="sm:hidden">Legal:</span>
                  <span className="hidden sm:inline">Legal cards:</span>
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
        ref={viewportRef}
        className={cn(
          'relative w-full pb-2 pt-4 overflow-y-hidden',
          needsScroll ? 'overflow-x-auto' : 'overflow-x-hidden'
        )}
        style={{ minHeight }}
      >
        {viewerHand.length === 0 ? (
          <div className="flex h-full items-center justify-center">
            <span className="text-sm text-subtle">
              Hand will appear once available.
            </span>
          </div>
        ) : (
          <div className="relative flex h-full items-center">
            {viewerHand.map((card, index) => {
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
                    'absolute focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/60 disabled:cursor-not-allowed',
                    'transition-[transform,scale,z-index] duration-300 ease-out',
                    'hover:z-10 hover:-translate-y-1 hover:scale-105',
                    isSelected
                      ? 'z-20 -translate-y-2 scale-110'
                      : isPlayable && viewerTurn
                        ? 'z-0'
                        : ''
                  )}
                  style={{
                    left: baseOffset + index * cardStep,
                    transform: isSelected
                      ? `translateY(-8px) scale(1.1)`
                      : undefined,
                  }}
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
            })}
          </div>
        )}
      </div>
    </section>
  )
}
