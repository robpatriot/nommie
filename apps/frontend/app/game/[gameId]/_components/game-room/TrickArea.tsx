import { useMemo } from 'react'
import { useTranslations } from 'next-intl'
import type {
  Card,
  PhaseSnapshot,
  RoundPublic,
  Seat,
} from '@/lib/game-room/types'
import { PlayingCard, CARD_DIMENSIONS } from './PlayingCard'
import { LastTrickCards } from './LastTrickCards'
import { SyncButton } from './SyncButton'
import { cn } from '@/lib/cn'
import { useMediaQuery } from '@/hooks/useMediaQuery'
import { shortenNameForDisplay, getOrientation, getActiveSeat } from './utils'

// Dimension constants
const CARD_HEIGHT = CARD_DIMENSIONS.md.height
const CARD_WIDTH = CARD_DIMENSIONS.md.width
const LABEL_HEIGHT = 20
const Z_INDEX_BASE = 20

// Sync button dimensions: py-1.5 (6px) + icon h-4 (16px) + py-1.5 (6px) = 28px total
const SYNC_BUTTON_HEIGHT = 28
const DIAMOND_LAYOUT_PADDING = 16 // Equal top and bottom padding for >640px diamond layout

// Card transform multipliers
const VERTICAL_OFFSET_MULTIPLIER = 0.1 // 10% of card height for top/bottom positioning
const HORIZONTAL_OFFSET_MULTIPLIER = 0.6 // 60% of card width for left/right positioning

// Gap and padding constants
const CARD_LABEL_GAP = 13 // Fixed gap between card and label (absolute value, not scaled)
const MOBILE_TOP_PADDING = 8 // Top padding for <640px
const MOBILE_BOTTOM_PADDING = 6 // Bottom padding for <640px
const SYNC_BUTTON_RIGHT_PADDING = 16 // Right padding for sync button (right-4 = 16px)
const MAX_NAME_LENGTH = 8 // Maximum length for shortened player names in trick area

// Diamond layout constants (>640px)
const DIAMOND_VERTICAL_OFFSET = 30 // Vertical offset for top/bottom cards in diamond layout
const DIAMOND_HORIZONTAL_OFFSET = 34 // Horizontal offset for left/right cards in diamond layout
const DIAMOND_CONTAINER_SIZE = 154 // Size of the diamond layout container (h-[154px] w-[154px])
const DIAMOND_MAX_RANDOM_OFFSET = 5 // Maximum random offset (in pixels) for card misalignment

// Transform constants
const CENTER_TRANSFORM = 'translate(-50%, -50%)' // Base transform for centering absolutely positioned elements

type Orientation = 'top' | 'bottom' | 'left' | 'right'

/**
 * Maps play order to fixed position layout.
 * First card (playOrder 0) = left, second (1) = top, third (2) = right, fourth (3) = bottom.
 */
function getFixedPosition(playOrder: number): Orientation {
  switch (playOrder) {
    case 0:
      return 'left'
    case 1:
      return 'top'
    case 2:
      return 'right'
    case 3:
      return 'bottom'
    default:
      return 'bottom'
  }
}

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
 * Builds the CSS transform string for a card based on its orientation and scale.
 * Offsets are calculated as proportions of card dimensions:
 * - Top/bottom: VERTICAL_OFFSET_MULTIPLIER (10%) of card height (scaled)
 * - Left/right: HORIZONTAL_OFFSET_MULTIPLIER (60%) of card width (scaled)
 *
 * @param orientation - Card orientation (left, top, right, bottom)
 * @param cardScale - Scale factor (should be pre-clamped via getSafeCardScale)
 * @param scaledCardWidth - Scaled card width
 * @param scaledCardHeight - Scaled card height
 * @returns CSS transform string, or 'none' if no transforms are needed
 */
function buildCardTransform(
  orientation: Orientation,
  cardScale: number,
  scaledCardWidth: number,
  scaledCardHeight: number
): string {
  const transforms: string[] = []

  // Calculate offsets as proportions of scaled card dimensions
  const verticalOffset = scaledCardHeight * VERTICAL_OFFSET_MULTIPLIER
  const horizontalOffset = scaledCardWidth * HORIZONTAL_OFFSET_MULTIPLIER

  if (orientation === 'top') {
    transforms.push(`translateY(${-verticalOffset}px)`)
  } else if (orientation === 'bottom') {
    transforms.push(`translateY(${verticalOffset}px)`)
  } else if (orientation === 'left') {
    transforms.push(`translateX(${-horizontalOffset}px)`)
  } else if (orientation === 'right') {
    transforms.push(`translateX(${horizontalOffset}px)`)
  }

  // Apply scale transform (applied last)
  if (cardScale !== 1) {
    transforms.push(`scale(${cardScale})`)
  }

  return transforms.length > 0 ? transforms.join(' ') : 'none'
}

interface ScaledDimensions {
  cardHeight: number
  cardWidth: number
  labelHeight: number
  gap: number
}

/**
 * Calculates all scaled dimensions based on cardScale.
 *
 * The scaling logic ensures:
 * - Cards and labels scale proportionally with cardScale
 * - Gap between card and label is fixed (not scaled)
 *
 * @param cardScale - Scale factor (0 < cardScale <= 1). Will be clamped to valid range.
 * @returns Object containing all scaled dimension values
 */
function calculateScaledDimensions(cardScale: number): ScaledDimensions {
  // Clamp cardScale to valid range to prevent division by zero and invalid values
  // Expected range: 0 < cardScale <= 1
  const safeCardScale = getSafeCardScale(cardScale)

  // Scale dimensions with cardScale proportionally
  const scaledCardHeight = CARD_HEIGHT * safeCardScale
  const scaledCardWidth = CARD_WIDTH * safeCardScale
  const scaledLabelHeight = LABEL_HEIGHT * safeCardScale

  // Fixed gap between card and label (absolute value, not scaled)
  const scaledGap = CARD_LABEL_GAP

  return {
    cardHeight: scaledCardHeight,
    cardWidth: scaledCardWidth,
    labelHeight: scaledLabelHeight,
    gap: scaledGap,
  }
}

/**
 * Calculates the minimum height required for the trick area container content.
 *
 * Height accounts for cards at north and south positions:
 * - Distance between top card (north) and bottom card (south) positions
 * - Card height
 * - Gap between card and label
 * - Label height
 *
 * (Padding is applied separately via inline styles)
 *
 * @param scaledDimensions - Scaled dimension values
 * @param isLargeViewport - Whether viewport is > 640px (determines offset calculation)
 * @returns Calculated height in pixels based on card dimensions (excluding padding)
 */
function calculateContainerHeight(
  scaledDimensions: ScaledDimensions,
  isLargeViewport: boolean
): number {
  if (isLargeViewport) {
    // For >640px diamond layout: distance between north and south card positions
    // Top card at -DIAMOND_VERTICAL_OFFSET, bottom card at +DIAMOND_VERTICAL_OFFSET
    // Cards are centered at their positions, so they extend cardHeight/2 above and below
    // Total height needed: 2 * DIAMOND_VERTICAL_OFFSET (offset distance) + scaled cardHeight
    const offsetBetweenTopAndBottom = DIAMOND_VERTICAL_OFFSET * 2
    return offsetBetweenTopAndBottom + scaledDimensions.cardHeight
  } else {
    // For <640px: Cards are centered at 50% and top card moves up by 10% of card height
    // Top card top edge = 50% of wrapper - 10% offset - 50% (half card) = 50% - 60% of card height
    // For top edge to be at 0: 50% of wrapper = 60% of card height
    // So center is at: 60% of card height
    // Card extends 50% below center, so card bottom = 60% + 50% = 110% of card height
    // Label bottom = card bottom + gap + labelHeight = 110% of card height + gap + labelHeight
    // Container height = 110% of card height + gap + labelHeight
    // Height = cardHeight + cardOffset + gap + labelHeight + syncButtonHeight + 2x topOffset + bottomPadding
    const verticalOffset =
      scaledDimensions.cardHeight * VERTICAL_OFFSET_MULTIPLIER
    return (
      scaledDimensions.cardHeight +
      verticalOffset +
      scaledDimensions.gap +
      scaledDimensions.labelHeight +
      SYNC_BUTTON_HEIGHT +
      MOBILE_TOP_PADDING * 2 +
      MOBILE_BOTTOM_PADDING
    )
  }
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

function getWaitingMessage(
  phase: PhaseSnapshot,
  getSeatName: (seat: Seat) => string,
  t: (key: string, params?: { name: string }) => string
): string {
  const activeSeat = getActiveSeat(phase)
  if (activeSeat === null) {
    return t('waitingForLead')
  }
  const activeName = getSeatName(activeSeat)
  switch (phase.phase) {
    case 'Bidding':
      return t('waitingForBidding', { name: activeName })
    case 'TrumpSelect':
      return t('waitingForTrump', { name: activeName })
    case 'Trick':
      return t('waitingForLead', { name: activeName })
    default:
      return t('waitingForLead')
  }
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
  const t = useTranslations('game.gameRoom.trickArea')
  const orderedCards = useMemo(() => {
    // Create a map of seat to play order index from phase.data.current_trick
    const playOrderMap = new Map<Seat, number>()
    if (phase.phase === 'Trick') {
      phase.data.current_trick.forEach(([seat], playIndex) => {
        playOrderMap.set(seat, playIndex)
      })
    }

    const cards = Array.from(trickMap.entries()).map(([seat, card]) => {
      const playOrder = playOrderMap.get(seat) ?? 0
      const fullName = getSeatName(seat)
      return {
        seat,
        card,
        label: shortenNameForDisplay(fullName, MAX_NAME_LENGTH),
        playOrder,
        fixedPosition: getFixedPosition(playOrder),
      }
    })
    return cards.slice().sort((a, b) => a.seat - b.seat)
  }, [trickMap, getSeatName, phase])

  // Check if viewport is > 640px for responsive top padding
  const isLargeViewport = useMediaQuery('(min-width: 640px)')

  // Generate random offsets for diamond layout cards based on trick number
  // Offsets change when trick number changes to create subtle misalignment
  const trickNo = phase.phase === 'Trick' ? phase.data.trick_no : 0
  const diamondOffsets = useMemo(() => {
    // Seed random generator with trick number for consistency
    // Simple seeded random function (linear congruential generator)
    // Generate seeds for each random value call without mutation
    const baseSeed = trickNo * 12345 + 67890

    const getRandom = (callIndex: number): number => {
      // Calculate seed for this specific call index
      let seed = baseSeed
      for (let i = 0; i < callIndex; i++) {
        seed = (seed * 1103515245 + 12345) & 0x7fffffff
      }
      const nextSeed = (seed * 1103515245 + 12345) & 0x7fffffff
      return nextSeed / 0x7fffffff
    }

    // Generate offsets for top/bottom cards (horizontal movement: left/right)
    const topDirection = getRandom(0) < 0.5 ? -1 : 1 // -1 for left, 1 for right
    const topAmount = Math.floor(getRandom(1) * DIAMOND_MAX_RANDOM_OFFSET) + 1 // 1 to DIAMOND_MAX_RANDOM_OFFSET px

    const bottomDirection = getRandom(2) < 0.5 ? -1 : 1
    const bottomAmount =
      Math.floor(getRandom(3) * DIAMOND_MAX_RANDOM_OFFSET) + 1 // 1 to DIAMOND_MAX_RANDOM_OFFSET px

    // Generate offsets for left/right cards (vertical movement: up/down)
    const leftDirection = getRandom(4) < 0.5 ? -1 : 1 // -1 for up, 1 for down
    const leftAmount = Math.floor(getRandom(5) * DIAMOND_MAX_RANDOM_OFFSET) + 1 // 1 to DIAMOND_MAX_RANDOM_OFFSET px

    const rightDirection = getRandom(6) < 0.5 ? -1 : 1
    const rightAmount = Math.floor(getRandom(7) * DIAMOND_MAX_RANDOM_OFFSET) + 1 // 1 to DIAMOND_MAX_RANDOM_OFFSET px

    // If top and bottom have same direction and amount, reverse bottom
    const finalTop = topDirection * topAmount
    const finalBottom =
      topDirection === bottomDirection && topAmount === bottomAmount
        ? -bottomDirection * bottomAmount
        : bottomDirection * bottomAmount

    // If left and right have same direction and amount, reverse right
    const finalLeft = leftDirection * leftAmount
    const finalRight =
      leftDirection === rightDirection && leftAmount === rightAmount
        ? -rightDirection * rightAmount
        : rightDirection * rightAmount

    return {
      top: finalTop,
      bottom: finalBottom,
      left: finalLeft,
      right: finalRight,
    }
  }, [trickNo])

  // Clamp cardScale once and reuse throughout
  const safeCardScale = getSafeCardScale(cardScale)

  // Calculate all scaled dimensions based on cardScale
  const scaledDimensions = calculateScaledDimensions(safeCardScale)

  // Determine if we should show last trick
  const isBetweenRounds =
    phase.phase === 'Bidding' || phase.phase === 'TrumpSelect'
  const hasCards = orderedCards.length > 0
  // Calculate container height - always based on card dimensions to prevent layout jumps
  const calculatedHeight = calculateContainerHeight(
    scaledDimensions,
    isLargeViewport
  )

  // Calculate padding values
  const paddingTop = isLargeViewport
    ? DIAMOND_LAYOUT_PADDING
    : MOBILE_TOP_PADDING
  const paddingRight = isLargeViewport ? undefined : SYNC_BUTTON_RIGHT_PADDING
  const paddingBottom = isLargeViewport
    ? DIAMOND_LAYOUT_PADDING
    : MOBILE_BOTTOM_PADDING

  const showLastTrick = shouldShowLastTrick(
    showPreviousRoundPosition,
    isBetweenRounds,
    hasCards,
    lastTrick
  )

  return (
    <div
      className={cn(
        'relative flex items-center justify-center rounded-[32px] border border-white/10 bg-black/25 px-4 text-center text-sm text-muted shadow-elevated backdrop-blur',
        className
      )}
      style={{
        // With border-box, height includes padding, so add padding to calculatedHeight
        height: `${calculatedHeight + paddingTop + paddingBottom}px`,
        paddingTop: `${paddingTop}px`,
        paddingBottom: `${paddingBottom}px`,
        ...(paddingRight !== undefined && {
          paddingRight: `${paddingRight}px`,
        }),
      }}
      role="region"
      aria-label={t('ariaLabel')}
    >
      {onRefresh ? (
        <div
          className="pointer-events-auto absolute z-10 sm:hidden"
          style={{
            top: `${paddingTop}px`,
            right: `${paddingRight ?? SYNC_BUTTON_RIGHT_PADDING}px`,
          }}
        >
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
            {getWaitingMessage(phase, getSeatName, t)}
          </span>
          {phase.phase === 'Trick' ? (
            <span className="text-xs text-muted">
              {t('trickOf', {
                trickNo: phase.data.trick_no,
                handSize: round?.hand_size ?? '?',
              })}
            </span>
          ) : null}
        </div>
      ) : isLargeViewport ? (
        // Large viewport: use diamond layout like LastTrickCards (without names)
        <div className="relative flex min-h-[200px] items-center justify-center overflow-visible py-8">
          <div
            className="relative mx-auto"
            style={{
              height: `${DIAMOND_CONTAINER_SIZE}px`,
              width: `${DIAMOND_CONTAINER_SIZE}px`,
            }}
          >
            {phase.phase === 'Trick' &&
              phase.data.current_trick.map(([seat, card], playOrder) => {
                const orientation = getOrientation(viewerSeat, seat)
                let positionTransform = CENTER_TRANSFORM
                switch (orientation) {
                  case 'top':
                    positionTransform = `${CENTER_TRANSFORM} translateY(-${DIAMOND_VERTICAL_OFFSET}px) translateX(${diamondOffsets.top}px)`
                    break
                  case 'bottom':
                    positionTransform = `${CENTER_TRANSFORM} translateY(${DIAMOND_VERTICAL_OFFSET}px) translateX(${diamondOffsets.bottom}px)`
                    break
                  case 'left':
                    positionTransform = `${CENTER_TRANSFORM} translateX(-${DIAMOND_HORIZONTAL_OFFSET}px) translateY(${diamondOffsets.left}px)`
                    break
                  case 'right':
                    positionTransform = `${CENTER_TRANSFORM} translateX(${DIAMOND_HORIZONTAL_OFFSET}px) translateY(${diamondOffsets.right}px)`
                    break
                }
                const scaleTransform =
                  safeCardScale !== 1 ? ` scale(${safeCardScale})` : ''
                const combinedTransform = `${positionTransform}${scaleTransform}`

                return (
                  <div
                    key={`${seat}-${playOrder}`}
                    className="absolute left-1/2 top-1/2 transition-all duration-300"
                    style={{
                      zIndex: Z_INDEX_BASE + playOrder,
                      transform: combinedTransform,
                      transformOrigin: 'center center',
                    }}
                  >
                    <PlayingCard card={card} size="md" />
                  </div>
                )
              })}
          </div>
        </div>
      ) : (
        <div
          className="relative overflow-visible px-2"
          style={{ height: `${calculatedHeight}px` }}
        >
          {(() => {
            // Calculate positioning values once (same for all cards)
            // Center of entire element (card + gap + label) should be positioned so visible gap above top card
            // is 2 * paddingTop + syncButtonHeight (for <640px only)
            // After translateY(-verticalOffset), center = centerTop - verticalOffset
            // Top edge = center - (cardHeight + gap + labelHeight) / 2
            // For top edge = visibleGap: centerTop - verticalOffset - (cardHeight + gap + labelHeight) / 2 = visibleGap
            // So: centerTop = visibleGap + verticalOffset + (cardHeight + gap + labelHeight) / 2
            const verticalOffset =
              scaledDimensions.cardHeight * VERTICAL_OFFSET_MULTIPLIER
            const totalElementHeight =
              scaledDimensions.cardHeight +
              scaledDimensions.gap +
              scaledDimensions.labelHeight
            const visibleGap = paddingTop * 2 + SYNC_BUTTON_HEIGHT
            const centerTop =
              visibleGap + verticalOffset + totalElementHeight / 2

            return orderedCards.map(
              ({ seat, card, label, fixedPosition, playOrder }) => {
                const positionTransform = buildCardTransform(
                  fixedPosition,
                  safeCardScale,
                  scaledDimensions.cardWidth,
                  scaledDimensions.cardHeight
                )
                const combinedTransform =
                  positionTransform === 'none'
                    ? CENTER_TRANSFORM
                    : `${CENTER_TRANSFORM} ${positionTransform}`

                return (
                  <div
                    key={seat}
                    className="absolute left-1/2 flex flex-col items-center transition-all duration-300"
                    style={{
                      top: `${centerTop}px`,
                      zIndex: Z_INDEX_BASE + playOrder,
                      transform: combinedTransform,
                      transformOrigin: 'center center',
                      gap: `${scaledDimensions.gap}px`,
                    }}
                  >
                    <PlayingCard card={card} size="md" />
                    <span className="text-[10px] font-semibold uppercase tracking-[0.3em] text-foreground">
                      {label}
                    </span>
                  </div>
                )
              }
            )
          })()}
        </div>
      )}
    </div>
  )
}
