'use client'

import { useCallback, useLayoutEffect, useMemo, useRef, useState } from 'react'
import { useTranslations } from 'next-intl'
import type { Card, PhaseSnapshot, Seat } from '@/lib/game-room/types'
import type { GameRoomViewProps } from '../game-room-view'
import { cn } from '@/lib/cn'
import { PlayingCard, CARD_DIMENSIONS } from './PlayingCard'
import { useMediaQuery } from '@/hooks/useMediaQuery'
import { isTrickPhase as checkIsTrickPhase } from './phase-helpers'

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

  // Pause signals lifted from TrickArea via GameRoomView
  isTrickPaused?: boolean
  isViewerLeaderDuringPause?: boolean
}

const CARD_WIDTH = CARD_DIMENSIONS.md.width
const CARD_HEIGHT = CARD_DIMENSIONS.md.height
const CARD_PADDING = 8 // padding + border
const EFFECTIVE_CARD_WIDTH = CARD_WIDTH + CARD_PADDING

const BLEND_THRESHOLD = 20 // Pixels of remaining space before we start blending
// Second row overlaps top row by 40% of card height
// Increased to account for card shadows that create visual gap
const ROW_OVERLAP = CARD_HEIGHT * 0.4
const SELECTED_CARD_LIFT = 8 // How much selected card lifts (translateY)
const MIN_SCALE = 0.75

// Import from new engine
import { computeLayout } from './layout-engine'

/**
 * LayoutEngine Integration
 * Defines how visual selection/geometry is computed from the abstract LayoutStructure
 */

interface CardPosition {
  left: number
  top: number
  zIndex: number
  scale: number
}

// Calculate overlap for a single row given viewport width and card count
function calculateOverlapForRow(
  viewportWidth: number,
  cardCount: number
): number {
  const MIN_OVERLAP = 4
  const MAX_OVERLAP = CARD_WIDTH - 17
  const AESTHETIC_OVERLAP = 8

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

function calculateRowGeometry(
  cards: Card[],
  viewportWidth: number,
  yOffset: number,
  baseZIndex: number
): { positions: Map<Card, CardPosition>; width: number } {
  const count = cards.length
  if (count === 0) return { positions: new Map(), width: 0 }

  const overlap = calculateOverlapForRow(viewportWidth, count)
  const cardStep = EFFECTIVE_CARD_WIDTH - overlap
  const totalWidth = EFFECTIVE_CARD_WIDTH + (count - 1) * cardStep
  const startLeft = (viewportWidth - totalWidth) / 2

  const positions = new Map<Card, CardPosition>()
  cards.forEach((card, i) => {
    positions.set(card, {
      left: startLeft + i * cardStep,
      top: yOffset,
      zIndex: baseZIndex + i,
      scale: 1,
    })
  })

  return { positions, width: totalWidth }
}

function computeLayoutPositions(
  cards: Card[],
  viewportWidth: number
): {
  positions: Map<Card, CardPosition>
  minHeight: number
  scale: number
} {
  // 1. Get logical structure (Search/Decision Tree)
  const structure = computeLayout(cards, viewportWidth)
  const isSingleRow = structure.bottomRow.length === 0

  // 2. Geometry & Scalling
  const getRequiredWidth = (n: number) => {
    const MAX_OVERLAP = CARD_WIDTH - 17
    if (n <= 1) return EFFECTIVE_CARD_WIDTH
    return EFFECTIVE_CARD_WIDTH + (n - 1) * (EFFECTIVE_CARD_WIDTH - MAX_OVERLAP)
  }

  const topN = structure.topRow.length
  const botN = structure.bottomRow.length
  const reqTop = getRequiredWidth(topN)
  const reqBot = getRequiredWidth(botN)
  const maxReq = Math.max(reqTop, reqBot)

  let scale = 1
  if (viewportWidth < maxReq) {
    scale = Math.max(MIN_SCALE, viewportWidth / maxReq)
  }

  // Effective viewport for calc
  const effectiveWidth = viewportWidth / scale

  // Calculate final positions
  const topGeo = calculateRowGeometry(structure.topRow, effectiveWidth, 0, 0)
  const botGeo = calculateRowGeometry(
    structure.bottomRow,
    effectiveWidth,
    CARD_HEIGHT - ROW_OVERLAP,
    100
  )

  const combined = new Map<Card, CardPosition>()
  // Merge Top
  topGeo.positions.forEach((pos, card) => {
    combined.set(card, { ...pos, scale }) // Apply global scale
  })
  // Merge Bottom
  botGeo.positions.forEach((pos, card) => {
    combined.set(card, { ...pos, scale })
  })

  const height = isSingleRow
    ? CARD_HEIGHT + 16 + SELECTED_CARD_LIFT
    : CARD_HEIGHT + (CARD_HEIGHT - ROW_OVERLAP) + 16 + SELECTED_CARD_LIFT

  return {
    positions: combined,
    minHeight: height,
    scale,
  }
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
  viewportRef: externalViewportRef,
  isTrickPaused = false,
  isViewerLeaderDuringPause = false,
}: PlayerHandProps) {
  const t = useTranslations('game.gameRoom.play')
  const tHand = useTranslations('game.gameRoom.hand')
  const tSuitAbbrev = useTranslations('game.gameRoom.hand.suitAbbrev')

  const isTrickPhase = checkIsTrickPhase(phase) && !!playState
  const isPreview = !isTrickPhase || !playState
  const isImmediatePlay = !requireCardConfirmation

  // "Turn" comes from the backend phase snapshot.
  const viewerTurn =
    isTrickPhase && playState
      ? phase.data.to_act === playState.viewerSeat
      : false

  // Pause-aware ability to act: if paused, only the leader is allowed to play through.
  const canActNow = isTrickPhase
    ? viewerTurn && (!isTrickPaused || isViewerLeaderDuringPause)
    : false

  const playableCards = useMemo(
    () => new Set(playState?.playable ?? []),
    [playState]
  )

  const waitingOnSeat = isTrickPhase ? phase.data.to_act : null
  const waitingOnName =
    waitingOnSeat === null
      ? null
      : waitingOnSeat === viewerSeat
        ? tHand('you')
        : playerNames[waitingOnSeat]

  const readOnlyPreviewText = tHand('status.readOnlyPreview')

  const handStatus = useMemo(() => {
    if (!viewerHand.length) {
      return t('status.handWillAppear')
    }

    // Immediate-play mode gets a special status (and a waiting message when it's not your turn).
    if (!isPreview && isImmediatePlay) {
      if (canActNow) {
        return t('status.tapToPlayImmediate')
      }

      // If it's our turn but we can't act, it must be due to a pause (and not being the leader).
      if (viewerTurn) {
        return tHand('status.pausing')
      }

      return waitingOnName
        ? t('status.waitingFor', { name: waitingOnName })
        : t('status.waitingForNext')
    }

    // Confirm mode + preview mode both default to read-only preview status text.
    return readOnlyPreviewText
  }, [
    viewerHand.length,
    isPreview,
    isImmediatePlay,
    canActNow,
    waitingOnName,
    t,
    tHand,
    viewerTurn,
    readOnlyPreviewText,
  ])

  // Title should only be hidden due to viewport size.
  const shouldShowTitle = useMediaQuery('(min-width: 400px)')
  const showLegalPlays = useMediaQuery('(min-width: 320px)')

  // Legal value display rules:
  // Show legal value when:
  //   - paused AND viewer is allowed to play through pause (leader), OR
  //   - not paused AND the leader has played (current_trick has cards)
  const leaderPlayed = isTrickPhase && phase.data.current_trick.length > 0
  const shouldShowLegalValue =
    (isViewerLeaderDuringPause && viewerTurn) ||
    (!isTrickPaused && leaderPlayed)

  const legalValue = useMemo(() => {
    if (!isTrickPhase || !playState) {
      return '—'
    }

    if (!shouldShowLegalValue) {
      return '—'
    }

    // During pause, if viewer is allowed to act (leader), show "all" (don’t reveal suit).
    if (isViewerLeaderDuringPause && viewerTurn) {
      return tHand('legal.all')
    }

    // Not paused; leader has played -> compute suit/all from visible lead.
    const leadCard = phase.data.current_trick[0]?.[1] ?? null
    if (!leadCard) {
      return tHand('legal.all')
    }

    const leadSuit = leadCard.slice(-1).toUpperCase()
    const hasLeadSuit = viewerHand.some(
      (card) => card.slice(-1).toUpperCase() === leadSuit
    )

    if (hasLeadSuit) {
      return tSuitAbbrev(leadSuit as 'S' | 'C' | 'H' | 'D')
    }

    return tHand('legal.all')
  }, [
    isTrickPhase,
    playState,
    shouldShowLegalValue,
    isViewerLeaderDuringPause,
    viewerTurn,
    phase,
    viewerHand,
    tSuitAbbrev,
    tHand,
  ])

  const internalViewportRef = useRef<HTMLDivElement | null>(null)
  const viewportRef = externalViewportRef ?? internalViewportRef

  // NOTE: Simple state structure now: just Map<Card, Position> + metadata
  const [layout, setLayout] = useState<{
    positions: Map<Card, CardPosition>
    minHeight: number
    scale: number
  }>({
    positions: new Map(),
    minHeight: CARD_HEIGHT + 16,
    scale: 1,
  })

  // Clear card suit cache when hand changes (if we still used one, but we don't. Kept for safety if external deps exist?)
  // Actually we deleted the SuitCache logic.
  // We can delete this effect.

  useLayoutEffect(() => {
    const viewport = viewportRef.current
    if (!viewport) {
      return
    }

    const updateLayout = () => {
      const width = viewport.clientWidth
      const newLayout = computeLayoutPositions(viewerHand, width)
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
  }, [viewerHand, viewportRef])

  const handleCardClick = useCallback(
    (card: Card) => {
      if (!isTrickPhase || !playState) {
        return
      }

      const isPlayable = playableCards.has(card)
      if (!canActNow || !isPlayable || playState.isPending) {
        return
      }

      if (isImmediatePlay) {
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
      canActNow,
      isImmediatePlay,
      onSelectCard,
      onPlayCard,
      selectedCard,
    ]
  )

  return (
    <section
      className={cn(
        'flex w-full flex-col gap-3 rounded-[28px] border border-border/70 bg-card/80 p-4 text-foreground shadow-elevated backdrop-blur',
        className
      )}
    >
      <header className="flex items-center gap-3">
        {/* Title container always exists; viewport alone decides visibility */}
        <div className="flex flex-col gap-1 min-w-0">
          {shouldShowTitle && (
            <span className="text-[11px] font-semibold uppercase tracking-[0.4em] text-muted-foreground break-words">
              {tHand('title')}
            </span>
          )}

          {shouldShowTitle && handStatus !== readOnlyPreviewText && (
            <p
              className="text-xs text-muted-foreground break-words"
              aria-live="polite"
            >
              {handStatus}
            </p>
          )}
        </div>

        {/* Confirmation-mode play button */}
        {isTrickPhase && playState && requireCardConfirmation ? (
          <div className="flex justify-center flex-1 min-w-0">
            {(() => {
              const state:
                | 'pending'
                | 'canAct'
                | 'pausing'
                | 'waitingFor'
                | 'waitingNext' = playState.isPending
                ? 'pending'
                : canActNow
                  ? 'canAct'
                  : viewerTurn
                    ? 'pausing'
                    : waitingOnName
                      ? 'waitingFor'
                      : 'waitingNext'

              let ariaLabel: string
              let content: React.ReactNode

              switch (state) {
                case 'pending': {
                  ariaLabel = t('button.aria.playing')
                  content = t('button.playing')
                  break
                }

                case 'canAct': {
                  ariaLabel = selectedCard
                    ? t('button.aria.playSelected', { card: selectedCard })
                    : t('button.aria.selectCard')

                  content = (
                    <>
                      <span className="sm:hidden">{t('button.playCard')}</span>
                      <span className="hidden sm:inline">
                        {t('button.playSelectedCard')}
                      </span>
                    </>
                  )
                  break
                }

                case 'pausing': {
                  ariaLabel = tHand('status.pausing')
                  content = tHand('status.pausing')
                  break
                }

                case 'waitingFor': {
                  // Runtime + TS safety: this should never be null here, but guard anyway.
                  const name = waitingOnName ?? tHand('you')
                  ariaLabel = t('button.waitingFor', { name })
                  content = t('button.waitingFor', { name })
                  break
                }

                case 'waitingNext': {
                  ariaLabel = t('button.waitingForNext')
                  content = t('button.waitingForNext')
                  break
                }
              }

              return (
                <button
                  type="button"
                  data-selected-card-exempt
                  onClick={async () => {
                    if (onPlayCard && selectedCard && canActNow) {
                      await onPlayCard(selectedCard)
                      onSelectCard(null)
                    }
                  }}
                  disabled={
                    !canActNow ||
                    playState.isPending ||
                    !selectedCard ||
                    !playState.playable.includes(selectedCard)
                  }
                  className="rounded-2xl bg-primary px-4 py-1.5 text-sm font-semibold text-primary-foreground shadow-lg shadow-primary/40 transition hover:bg-primary/90 disabled:cursor-not-allowed disabled:bg-primary/40 disabled:text-primary-foreground/70"
                  aria-label={ariaLabel}
                >
                  {content}
                </button>
              )
            })()}
          </div>
        ) : null}

        {isTrickPhase && playState && (showLegalPlays || isImmediatePlay) ? (
          <div
            className="ml-auto flex items-center justify-end gap-2 flex-shrink-0"
            style={{ minWidth: 'max-content' }}
          >
            <div className="rounded-full bg-overlay/20 px-3 py-1">
              <span className="text-sm font-medium text-muted-foreground">
                <span className="sm:hidden">{t('legal.short')}</span>
                <span className="hidden sm:inline">{t('legal.long')}</span>
              </span>
              <span className="ml-1.5 text-sm font-medium text-foreground">
                {legalValue}
              </span>
            </div>
          </div>
        ) : null}
      </header>

      <div
        ref={viewportRef}
        className="relative w-full pb-2 pt-4 overflow-visible"
        style={{ minHeight: layout.minHeight }}
      >
        {viewerHand.length === 0 ? (
          <div className="flex h-full items-center justify-center">
            <span className="text-sm text-muted-foreground">
              {tHand('empty')}
            </span>
          </div>
        ) : (
          <div className="relative w-full h-full">
            {viewerHand.map((card) => {
              const isPlayable = playableCards.has(card)
              const isSelected = selectedCard === card
              const isDisabled =
                !isTrickPhase ||
                !playState ||
                !isPlayable ||
                !canActNow ||
                playState.isPending

              const cardLabel = isPlayable
                ? tHand('cardAria', {
                    card,
                    state: isSelected
                      ? tHand('cardState.selected')
                      : tHand('cardState.playable'),
                  })
                : tHand('cardAria', {
                    card,
                    state: isDisabled
                      ? tHand('cardState.notPlayable')
                      : tHand('cardState.playable'),
                  })

              // Lookup position from the Map
              const position = layout.positions.get(card)

              // Skip rendering if position not yet computed (initial render before useLayoutEffect runs)
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
                  data-selected-card-exempt
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
                  // Direct style manipulation is necessary here because each card has a unique baseZIndex
                  // that changes dynamically. CSS classes can't express per-instance z-index calculations.
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
