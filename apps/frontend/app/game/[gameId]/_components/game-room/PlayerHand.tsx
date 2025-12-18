'use client'

import { useCallback, useLayoutEffect, useMemo, useRef, useState } from 'react'
import type { Card, PhaseSnapshot, Seat } from '@/lib/game-room/types'
import type { GameRoomViewProps } from '../game-room-view'
import { cn } from '@/lib/cn'
import { PlayingCard, CARD_DIMENSIONS } from './PlayingCard'
import { useMediaQuery } from '@/hooks/useMediaQuery'

export type LayoutVariant = 'default' | 'scaled'

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
  layoutVariant?: LayoutVariant
  viewportRef?: React.RefObject<HTMLDivElement | null>
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
// Second row overlaps top row by 25% of card height
// Increased to account for card shadows that create visual gap
const ROW_OVERLAP = CARD_HEIGHT * 0.4
const SELECTED_CARD_LIFT = 8 // How much selected card lifts (translateY)

type LayoutMode = 'singleRow' | 'twoRow'

interface CardPosition {
  left: number
  top: number
  zIndex: number
}

interface LayoutResult {
  mode: LayoutMode
  positions: CardPosition[]
  minHeight: number
  scale?: number // Optional scale factor for card scaling
}

// Calculate overlap for a single row given viewport width and card count
function calculateOverlapForRow(
  viewportWidth: number,
  cardCount: number
): number {
  if (cardCount <= 1) {
    return 0
  }

  const totalWidthNeeded = EFFECTIVE_CARD_WIDTH * cardCount
  const spaceRemaining = viewportWidth - totalWidthNeeded

  if (spaceRemaining >= BLEND_THRESHOLD) {
    // Plenty of space: use aesthetic overlap
    return AESTHETIC_OVERLAP
  }

  if (spaceRemaining > 0) {
    // Tight but fits: blend between aesthetic and calculated
    const overlapNeeded = (totalWidthNeeded - viewportWidth) / (cardCount - 1)
    const calculatedOverlap = Math.max(
      MIN_OVERLAP,
      Math.min(MAX_OVERLAP, overlapNeeded)
    )
    const blend = 1 - spaceRemaining / BLEND_THRESHOLD
    return AESTHETIC_OVERLAP + (calculatedOverlap - AESTHETIC_OVERLAP) * blend
  }

  // Doesn't fit: calculate required overlap
  const overlapNeeded = (totalWidthNeeded - viewportWidth) / (cardCount - 1)
  return Math.max(MIN_OVERLAP, Math.min(MAX_OVERLAP, overlapNeeded))
}

// Calculate the minimum width needed for a row to reach max overlap
function calculateMinWidthForMaxOverlap(cardCount: number): number {
  if (cardCount <= 1) {
    return EFFECTIVE_CARD_WIDTH
  }
  return (
    EFFECTIVE_CARD_WIDTH +
    (cardCount - 1) * (EFFECTIVE_CARD_WIDTH - MAX_OVERLAP)
  )
}

// Check if cards fit in one row with acceptable overlap
function canFitInOneRow(viewportWidth: number, cardCount: number): boolean {
  if (cardCount <= 1) {
    return true
  }
  // If viewport is below absolute minimum, we need two rows
  return viewportWidth >= calculateMinWidthForMaxOverlap(cardCount)
}

/**
 * Layout strategy interface for PlayerHand layouts.
 * Each strategy computes card positions, dimensions, and optional scaling.
 */
interface LayoutStrategy {
  computeLayout(viewportWidth: number, cardCount: number): LayoutResult
}

/**
 * Layout engine for PlayerHand that avoids horizontal scrolling entirely.
 *
 * Layout modes:
 * - singleRow: Cards arranged in one centered row with overlap. Used when cards
 *   fit within viewport width using acceptable overlap (respects MAX_OVERLAP ceiling).
 * - twoRow: Cards split into two balanced rows when single-row would exceed overlap
 *   limits. Each row is independently centered and uses its own overlap calculation.
 *
 * Design principles:
 * - No horizontal scrolling: overflow-x is always hidden. When cards don't fit in one
 *   row, we switch to two-row layout rather than enabling scrolling.
 * - Deterministic positioning: All positions computed from viewport width and card
 *   count, avoiding DOM measurement jitter during resize/rotation.
 * - Visual effects preserved: Drop shadows and selected-card lift/scale are not
 *   clipped by using overflow-hidden (not overflow-y-hidden) and adequate minHeight.
 * - Stable z-index: Selected cards render above neighbors (including across rows)
 *   via deterministic z-index boost.
 */
class DefaultLayoutStrategy implements LayoutStrategy {
  computeLayout(viewportWidth: number, cardCount: number): LayoutResult {
    if (cardCount === 0) {
      return {
        mode: 'singleRow',
        positions: [],
        minHeight: CARD_HEIGHT + 16,
      }
    }

    // Check if we can fit in one row
    if (canFitInOneRow(viewportWidth, cardCount)) {
      // Single row layout
      const overlap = calculateOverlapForRow(viewportWidth, cardCount)
      const cardStep = EFFECTIVE_CARD_WIDTH - overlap
      const stripWidth = EFFECTIVE_CARD_WIDTH + (cardCount - 1) * cardStep
      const baseOffset = (viewportWidth - stripWidth) / 2

      const positions: CardPosition[] = []
      for (let i = 0; i < cardCount; i++) {
        positions.push({
          left: baseOffset + i * cardStep,
          top: 0,
          zIndex: i, // Base z-index by index
        })
      }

      return {
        mode: 'singleRow',
        positions,
        minHeight: CARD_HEIGHT + 16 + SELECTED_CARD_LIFT,
      }
    }

    // Two row layout
    // Split cards into two rows (balanced by count)
    const topRowCount = Math.ceil(cardCount / 2)
    const bottomRowCount = cardCount - topRowCount

    // Calculate overlap for each row independently
    const topOverlap = calculateOverlapForRow(viewportWidth, topRowCount)
    const bottomOverlap =
      bottomRowCount > 0
        ? calculateOverlapForRow(viewportWidth, bottomRowCount)
        : 0

    const topCardStep = EFFECTIVE_CARD_WIDTH - topOverlap
    const bottomCardStep =
      bottomRowCount > 0 ? EFFECTIVE_CARD_WIDTH - bottomOverlap : 0

    // Center each row
    const topStripWidth = EFFECTIVE_CARD_WIDTH + (topRowCount - 1) * topCardStep
    const bottomStripWidth =
      bottomRowCount > 0
        ? EFFECTIVE_CARD_WIDTH + (bottomRowCount - 1) * bottomCardStep
        : 0

    const topBaseOffset = (viewportWidth - topStripWidth) / 2
    const bottomBaseOffset = (viewportWidth - bottomStripWidth) / 2

    const topRowY = 0
    const bottomRowY = CARD_HEIGHT - ROW_OVERLAP

    const positions: CardPosition[] = []
    for (let i = 0; i < cardCount; i++) {
      const isTopRow = i < topRowCount
      const rowIndex = isTopRow ? i : i - topRowCount

      // Z-index strategy: bottom row cards render above top row cards in overlap area
      // Within each row, later cards have higher z-index (for horizontal overlap)
      // Bottom row gets base 100, top row gets base 0
      const zIndex = isTopRow
        ? rowIndex // Top row: 0-99 range
        : 100 + rowIndex // Bottom row: 100-199 range

      positions.push({
        left: isTopRow
          ? topBaseOffset + rowIndex * topCardStep
          : bottomBaseOffset + rowIndex * bottomCardStep,
        top: isTopRow ? topRowY : bottomRowY,
        zIndex,
      })
    }

    return {
      mode: 'twoRow',
      positions,
      minHeight:
        CARD_HEIGHT + (CARD_HEIGHT - ROW_OVERLAP) + 16 + SELECTED_CARD_LIFT,
    }
  }
}

/**
 * Scaled layout strategy that applies card scaling below a threshold width.
 * When viewport is narrow, cards scale down to fit better while maintaining readability.
 * Maintains the same overlap ratio as full-size cards by scaling overlap proportionally.
 */
class ScaledLayoutStrategy implements LayoutStrategy {
  private readonly MIN_SCALE = 0.75 // Don't scale below this factor
  private readonly baseStrategy = new DefaultLayoutStrategy()

  computeLayout(viewportWidth: number, cardCount: number): LayoutResult {
    const baseLayout = this.baseStrategy.computeLayout(viewportWidth, cardCount)

    // Single row mode: no scaling, return base layout
    if (baseLayout.mode === 'singleRow') {
      return {
        ...baseLayout,
        scale: 1,
      }
    }

    // Two-row mode: check if scaling is needed
    const topRowCount = Math.ceil(cardCount / 2)
    const bottomRowCount = cardCount - topRowCount

    // Check overlaps with actual viewport to determine if max overlap is reached
    const topOverlap = calculateOverlapForRow(viewportWidth, topRowCount)
    const bottomOverlap =
      bottomRowCount > 0
        ? calculateOverlapForRow(viewportWidth, bottomRowCount)
        : 0

    const topRowAtMaxOverlap =
      topRowCount > 1 && Math.abs(topOverlap - MAX_OVERLAP) < 0.1
    const bottomRowAtMaxOverlap =
      bottomRowCount > 1 && Math.abs(bottomOverlap - MAX_OVERLAP) < 0.1
    const eitherRowAtMaxOverlap = topRowAtMaxOverlap || bottomRowAtMaxOverlap

    // Calculate the width where max overlap would be reached
    const minWidthForTwoRowMaxOverlap = Math.max(
      calculateMinWidthForMaxOverlap(topRowCount),
      calculateMinWidthForMaxOverlap(bottomRowCount)
    )

    // Determine scale: only scale when max overlap is reached and viewport is below threshold
    let scale = 1
    if (eitherRowAtMaxOverlap && viewportWidth < minWidthForTwoRowMaxOverlap) {
      scale = Math.max(
        this.MIN_SCALE,
        viewportWidth / minWidthForTwoRowMaxOverlap
      )
    }

    // If no scaling needed, return base layout
    if (scale === 1) {
      return {
        ...baseLayout,
        scale: 1,
      }
    }

    // Calculate positions using effective viewport width to maintain overlap ratio
    // Then scale positions back to actual viewport coordinates
    const effectiveViewportWidth = viewportWidth / scale
    const scaledLayout = this.baseStrategy.computeLayout(
      effectiveViewportWidth,
      cardCount
    )

    // Scale positions to actual viewport coordinates
    const scaledCardHeight = CARD_HEIGHT * scale
    const scaledSelectedLift = SELECTED_CARD_LIFT * scale
    const scaledRowOverlap = ROW_OVERLAP * scale

    // Determine which row each card is in for proper top position scaling
    const positions: CardPosition[] = scaledLayout.positions.map(
      (pos, index) => {
        const isTopRow = index < topRowCount
        return {
          ...pos,
          left: pos.left * scale,
          // Top row stays at 0, bottom row uses scaled overlap
          top: isTopRow ? 0 : scaledCardHeight - scaledRowOverlap,
        }
      }
    )

    return {
      mode: scaledLayout.mode,
      positions,
      minHeight:
        scaledCardHeight +
        (scaledCardHeight - scaledRowOverlap) +
        16 +
        scaledSelectedLift,
      scale,
    }
  }
}

// Strategy registry
const layoutStrategies: Record<LayoutVariant, LayoutStrategy> = {
  default: new DefaultLayoutStrategy(),
  scaled: new ScaledLayoutStrategy(),
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
  layoutVariant = 'default',
  viewportRef: externalViewportRef,
}: PlayerHandProps) {
  const isTrickPhase = phase.phase === 'Trick' && !!playState
  const viewerTurn = isTrickPhase && phase.data.to_act === playState!.viewerSeat
  const playableCards = useMemo(
    () => new Set(playState?.playable ?? []),
    [playState]
  )
  const waitingOnSeat = isTrickPhase ? phase.data.to_act : null
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
  } else if (viewerTurn && !requireCardConfirmation) {
    handStatus = 'Tap a legal card to play immediately.'
  }

  // Responsive visibility: hide title below 400px, legal plays below 320px
  const showTitle = useMediaQuery('(min-width: 400px)')
  const showLegalPlays = useMediaQuery('(min-width: 320px)')

  // Memoize legal cards display calculation
  const legalCardsDisplay = useMemo(() => {
    if (!isTrickPhase || !playState || !viewerTurn) {
      return null
    }

    // Get lead card if trick has started
    const leadCard =
      phase.phase === 'Trick' && phase.data.current_trick.length > 0
        ? phase.data.current_trick[0][1]
        : null

    if (leadCard) {
      // Rule 2: If player has cards matching lead suit, only that suit is legal
      const leadSuit = leadCard.slice(-1).toUpperCase()
      const hasLeadSuit = viewerHand.some(
        (card) => card.slice(-1).toUpperCase() === leadSuit
      )

      if (hasLeadSuit) {
        // Only one suit is legal (the lead suit)
        return leadSuit
      }
    }

    // Rule 3: No lead card or player doesn't have lead suit - all cards are legal
    return 'all'
  }, [isTrickPhase, playState, viewerTurn, phase, viewerHand])

  const internalViewportRef = useRef<HTMLDivElement | null>(null)
  const viewportRef = externalViewportRef ?? internalViewportRef
  const [layout, setLayout] = useState<LayoutResult>({
    mode: 'singleRow',
    positions: [],
    minHeight: CARD_HEIGHT + 16,
  })

  useLayoutEffect(() => {
    const viewport = viewportRef.current
    if (!viewport) {
      return
    }

    const strategy = layoutStrategies[layoutVariant] ?? layoutStrategies.default

    const updateLayout = () => {
      const width = viewport.clientWidth
      const newLayout = strategy.computeLayout(width, viewerHand.length)
      setLayout(newLayout)
    }

    updateLayout()

    const resizeObserver = new ResizeObserver(() => {
      updateLayout()
    })
    resizeObserver.observe(viewport)

    return () => {
      resizeObserver.disconnect()
    }
  }, [viewerHand.length, layoutVariant]) // eslint-disable-line react-hooks/exhaustive-deps -- viewportRef is a stable ref object

  const handleCardClick = useCallback(
    (card: Card) => {
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
    },
    [
      isTrickPhase,
      playState,
      playableCards,
      viewerTurn,
      requireCardConfirmation,
      onSelectCard,
      onPlayCard,
      selectedCard,
    ]
  )

  return (
    <section
      className={cn(
        'flex w-full flex-col gap-3 rounded-[28px] border border-white/15 bg-surface/80 p-4 text-foreground shadow-[0_35px_80px_rgba(0,0,0,0.4)] backdrop-blur',
        className
      )}
    >
      <header className="flex items-center gap-3">
        <div className="flex flex-col gap-1 flex-shrink-0">
          {showTitle && (
            <span className="text-[11px] font-semibold uppercase tracking-[0.4em] text-subtle">
              Your hand
            </span>
          )}
          {handStatus !== 'Read-only preview' && (
            <p className="text-xs text-muted" aria-live="polite">
              {handStatus}
            </p>
          )}
        </div>
        {isTrickPhase && playState ? (
          <div className="flex justify-center flex-1 min-w-0">
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
        ) : null}
        {isTrickPhase && playState && showLegalPlays ? (
          <div
            className="flex items-center justify-end gap-2 flex-shrink-0"
            style={{ minWidth: 'max-content' }}
          >
            {playState.playable.length > 0 &&
            viewerTurn &&
            legalCardsDisplay ? (
              <div className="rounded-full bg-black/20 px-3 py-1">
                <span className="text-sm font-medium text-muted">
                  <span className="sm:hidden">Legal:</span>
                  <span className="hidden sm:inline">Legal cards:</span>
                </span>
                <span className="ml-1.5 text-sm font-medium text-foreground">
                  {legalCardsDisplay}
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
        className="relative w-full pb-2 pt-4 overflow-visible"
        style={{ minHeight: layout.minHeight }}
      >
        {viewerHand.length === 0 ? (
          <div className="flex h-full items-center justify-center">
            <span className="text-sm text-subtle">
              Hand will appear once available.
            </span>
          </div>
        ) : (
          <div className="relative w-full h-full">
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

              const position = layout.positions[index]
              if (!position) {
                return null
              }

              // Selected card gets z-index boost to render above neighbors (including across rows)
              const baseZIndex = position.zIndex
              const zIndex = isSelected ? baseZIndex + 1000 : baseZIndex

              // Apply scale if present in layout result
              const scale = layout.scale ?? 1
              const baseTransform = `scale(${scale})`
              const selectedTransform = `translateY(-${SELECTED_CARD_LIFT * scale}px) scale(${scale * 1.1})`
              const hoverTransform = `translateY(-${1 * scale}px) scale(${scale * 1.05})`

              return (
                <button
                  key={card}
                  type="button"
                  onClick={() => handleCardClick(card)}
                  disabled={isDisabled}
                  className={cn(
                    'absolute focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/60 disabled:cursor-not-allowed',
                    'transition-[transform] duration-300 ease-out'
                  )}
                  style={{
                    left: position.left,
                    top: position.top,
                    zIndex,
                    transform: isSelected ? selectedTransform : baseTransform,
                    transformOrigin: 'top left',
                  }}
                  onMouseEnter={(e) => {
                    if (!isSelected) {
                      e.currentTarget.style.zIndex = String(baseZIndex + 50)
                      e.currentTarget.style.transform = hoverTransform
                    }
                  }}
                  onMouseLeave={(e) => {
                    if (!isSelected) {
                      e.currentTarget.style.zIndex = String(baseZIndex)
                      e.currentTarget.style.transform = baseTransform
                    }
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
