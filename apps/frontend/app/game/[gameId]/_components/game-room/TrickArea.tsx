import { useMemo } from 'react'
import type { PhaseSnapshot, RoundPublic, Seat } from '@/lib/game-room/types'
import type { Card } from '@/lib/game-room/types'
import { getOrientation, ORIENTATION_ORDER_TRICK } from './utils'
import { PlayingCard, CARD_DIMENSIONS } from './PlayingCard'
import { LastTrickCards } from './LastTrickCards'
import { SyncButton } from './SyncButton'
import { cn } from '@/lib/cn'
import { useMediaQuery } from '@/hooks/useMediaQuery'

// Dimension constants
const CARD_HEIGHT = CARD_DIMENSIONS.md.height
const LABEL_HEIGHT = 20
const TOP_PADDING_BASE = 60
const BOTTOM_PADDING = 16
const BASE_GAP = 8
const BASE_MARGIN = 32
const OVERLAP_MULTIPLIER = 3
const GAP_MULTIPLIER = 2
const PADDING_INCREASE_MULTIPLIER = 0.75
const Z_INDEX_BASE = 20

// Orientation offset constants
const ORIENTATION_OFFSETS = {
  top: -12,
  bottom: 12,
  left: -8,
  right: 8,
} as const

type Orientation = 'top' | 'bottom' | 'left' | 'right'

/**
 * Clamps cardScale to a valid range to prevent division by zero and invalid values.
 *
 * @param cardScale - Scale factor to clamp
 * @returns Clamped value between 0.01 and 1
 */
function getSafeCardScale(cardScale: number): number {
  return Math.max(0.01, Math.min(1, cardScale))
}

/**
 * Builds the CSS transform string for a card based on its orientation,
 * scale, and overlap adjustment.
 *
 * @param orientation - Card orientation relative to viewer
 * @param cardScale - Scale factor (should be pre-clamped via getSafeCardScale)
 * @param overlapAdjustment - Adjustment value to move cards closer together
 * @returns CSS transform string, or 'none' if no transforms are needed
 */
function buildCardTransform(
  orientation: Orientation,
  cardScale: number,
  overlapAdjustment: number
): string {
  const transforms: string[] = []

  if (orientation === 'top') {
    transforms.push(`translateY(${ORIENTATION_OFFSETS.top}px)`)
  } else if (orientation === 'bottom') {
    transforms.push(`translateY(${ORIENTATION_OFFSETS.bottom}px)`)
  } else if (orientation === 'left') {
    // Left card moves right (more positive) to get closer to center
    const totalX = ORIENTATION_OFFSETS.left + overlapAdjustment
    transforms.push(`translateX(${totalX}px)`)
  } else if (orientation === 'right') {
    // Right card moves left (more negative) to get closer to center
    const totalX = ORIENTATION_OFFSETS.right - overlapAdjustment
    transforms.push(`translateX(${totalX}px)`)
  }

  // Apply scale transform (applied last)
  if (cardScale !== 1) {
    transforms.push(`scale(${cardScale})`)
  }

  return transforms.length > 0 ? transforms.join(' ') : 'none'
}

interface ScaledDimensions {
  cardHeight: number
  labelHeight: number
  topPadding: number
  bottomPadding: number
  gap: number
  overlapAdjustment: number
}

/**
 * Calculates all scaled dimensions based on cardScale.
 *
 * The scaling logic ensures:
 * - Cards and labels scale proportionally with cardScale
 * - Top padding increases as cards scale down to maintain space for sync button
 * - Bottom padding scales down proportionally
 * - Gap between card and label increases as cards scale down (inverse relationship)
 * - Cards move closer together (overlap adjustment) as they scale down
 *
 * @param cardScale - Scale factor (0 < cardScale <= 1). Will be clamped to valid range.
 * @param topPadding - Base top padding value (already responsive to viewport)
 * @returns Object containing all scaled dimension values
 */
function calculateScaledDimensions(
  cardScale: number,
  topPadding: number
): ScaledDimensions {
  // Clamp cardScale to valid range to prevent division by zero and invalid values
  // Expected range: 0 < cardScale <= 1
  const safeCardScale = getSafeCardScale(cardScale)

  // Scale dimensions with cardScale proportionally
  const scaledCardHeight = CARD_HEIGHT * safeCardScale
  const scaledLabelHeight = LABEL_HEIGHT * safeCardScale
  const scaledBottomPadding = BOTTOM_PADDING * safeCardScale

  // Top padding increases as cards scale down to maintain space for sync button
  // Formula: basePadding + (basePadding * (1 - scale) * multiplier)
  // This creates an inverse relationship where smaller cards = more padding
  const scaleDifference = 1 - safeCardScale
  const paddingIncrease =
    topPadding * scaleDifference * PADDING_INCREASE_MULTIPLIER
  const scaledTopPadding = topPadding + paddingIncrease

  // Gap increases as cards scale down (for visual spacing between card and label)
  // Formula: (baseGap / scale) * multiplier when scale < 1
  // Safety check: ensure cardScale > 0 to prevent division by zero
  const scaledGap =
    safeCardScale > 0 && safeCardScale < 1
      ? (BASE_GAP / safeCardScale) * GAP_MULTIPLIER
      : BASE_GAP

  // Overlap adjustment - move cards closer together as they scale down
  // Formula: baseMargin * (1 - scale) * multiplier
  // Creates proportional movement toward center as scale decreases
  const overlapAdjustment =
    safeCardScale < 1
      ? BASE_MARGIN * (1 - safeCardScale) * OVERLAP_MULTIPLIER
      : 0

  return {
    cardHeight: scaledCardHeight,
    labelHeight: scaledLabelHeight,
    topPadding: scaledTopPadding,
    bottomPadding: scaledBottomPadding,
    gap: scaledGap,
    overlapAdjustment,
  }
}

/**
 * Calculates the minimum height required for the trick area container.
 *
 * Height = card height + label height + gap + top padding + bottom padding
 *
 * Always returns the height based on card dimensions to prevent layout jumps
 * when cards are added or removed.
 *
 * @param scaledDimensions - Scaled dimension values
 * @returns Calculated height in pixels based on card dimensions
 */
function calculateContainerHeight(scaledDimensions: ScaledDimensions): number {
  return (
    scaledDimensions.cardHeight +
    scaledDimensions.labelHeight +
    scaledDimensions.gap +
    scaledDimensions.topPadding +
    scaledDimensions.bottomPadding
  )
}

/**
 * Determines whether to show the last trick cards.
 *
 * Shows last trick when:
 * - showPreviousRoundPosition is explicitly true, OR
 * - We're between rounds (Bidding/TrumpSelect phase) AND there are no current cards
 *
 * @param showPreviousRoundPosition - Explicit flag to show previous round position
 * @param isBetweenRounds - Whether we're in Bidding or TrumpSelect phase
 * @param hasCurrentCards - Whether there are cards in the current trick
 * @param lastTrick - The last trick data (must exist and have cards)
 * @returns True if last trick should be displayed
 */
function shouldShowLastTrick(
  showPreviousRoundPosition: boolean | undefined,
  isBetweenRounds: boolean,
  hasCurrentCards: boolean,
  lastTrick: Array<[Seat, Card]> | null | undefined
): boolean {
  const shouldShow =
    showPreviousRoundPosition ?? (isBetweenRounds && !hasCurrentCards)
  return shouldShow && !!lastTrick && lastTrick.length > 0
}

interface TrickAreaProps {
  trickMap: Map<Seat, Card>
  getSeatName: (seat: Seat) => string
  round: RoundPublic | null
  phase: PhaseSnapshot
  viewerSeat: Seat
  lastTrick?: Array<[Seat, Card]> | null
  showPreviousRoundPosition?: boolean
  className?: string
  onRefresh?: () => void
  isRefreshing?: boolean
  cardScale?: number
}

export function TrickArea({
  trickMap,
  getSeatName,
  round,
  phase,
  viewerSeat,
  lastTrick,
  showPreviousRoundPosition,
  className = '',
  onRefresh,
  isRefreshing = false,
  cardScale = 1,
}: TrickAreaProps) {
  const orderedCards = useMemo(() => {
    const cards = Array.from(trickMap.entries()).map(([seat, card]) => ({
      seat,
      card,
      label: getSeatName(seat),
      orientation: getOrientation(viewerSeat, seat),
    }))
    return cards
      .slice()
      .sort(
        (a, b) =>
          ORIENTATION_ORDER_TRICK.indexOf(a.orientation) -
          ORIENTATION_ORDER_TRICK.indexOf(b.orientation)
      )
  }, [trickMap, getSeatName, viewerSeat])

  // Check if viewport is > 640px for responsive top padding
  const isLargeViewport = useMediaQuery('(min-width: 640px)')

  // Calculate responsive top padding
  const topPadding = isLargeViewport ? TOP_PADDING_BASE / 2 : TOP_PADDING_BASE

  // Clamp cardScale once and reuse throughout
  const safeCardScale = getSafeCardScale(cardScale)

  // Calculate all scaled dimensions based on cardScale
  const scaledDimensions = calculateScaledDimensions(safeCardScale, topPadding)

  // Determine if we should show last trick
  const isBetweenRounds =
    phase.phase === 'Bidding' || phase.phase === 'TrumpSelect'
  const hasCards = orderedCards.length > 0
  const showLastTrick = shouldShowLastTrick(
    showPreviousRoundPosition,
    isBetweenRounds,
    hasCards,
    lastTrick
  )

  // Calculate container height - always based on card dimensions to prevent layout jumps
  const calculatedHeight = calculateContainerHeight(scaledDimensions)

  return (
    <div
      className={cn(
        'relative flex items-center justify-center rounded-[32px] border border-white/10 bg-black/25 px-4 text-center text-sm text-muted shadow-[0_35px_90px_rgba(0,0,0,0.4)] backdrop-blur',
        className
      )}
      style={{
        height: `${calculatedHeight}px`,
        paddingTop: `${scaledDimensions.topPadding}px`,
        paddingBottom: `${scaledDimensions.bottomPadding}px`,
      }}
      role="region"
      aria-label="Current trick cards"
    >
      {onRefresh ? (
        <div className="pointer-events-auto absolute right-4 top-4 z-10 sm:hidden">
          <SyncButton onRefresh={onRefresh} isRefreshing={isRefreshing} />
        </div>
      ) : null}
      {showLastTrick && lastTrick ? (
        <div className="flex w-full flex-col gap-4">
          <LastTrickCards
            lastTrick={lastTrick}
            getSeatName={getSeatName}
            viewerSeat={viewerSeat}
            showNames={false}
          />
        </div>
      ) : orderedCards.length === 0 ? (
        <div className="flex flex-col items-center gap-2">
          <span className="text-sm font-medium text-subtle">
            Waiting for lead…
          </span>
          {phase.phase === 'Trick' ? (
            <span className="text-xs text-muted">
              Trick {phase.data.trick_no} of {round?.hand_size ?? '?'}
            </span>
          ) : null}
        </div>
      ) : (
        <div className="relative flex items-center justify-center gap-0 overflow-visible px-2">
          {orderedCards.map(({ seat, card, label, orientation }, index) => {
            const baseMarginLeft = index > 0 ? -BASE_MARGIN : 0
            const combinedTransform = buildCardTransform(
              orientation,
              safeCardScale,
              scaledDimensions.overlapAdjustment
            )

            return (
              <div
                key={seat}
                className="relative flex flex-col items-center transition-all duration-300"
                style={{
                  zIndex: Z_INDEX_BASE + index,
                  transform: combinedTransform,
                  transformOrigin: 'center center',
                  marginLeft: baseMarginLeft,
                  gap: `${scaledDimensions.gap}px`,
                }}
              >
                <PlayingCard card={card} size="md" />
                <span className="text-[10px] font-semibold uppercase tracking-[0.3em] text-foreground">
                  {label}
                </span>
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}
