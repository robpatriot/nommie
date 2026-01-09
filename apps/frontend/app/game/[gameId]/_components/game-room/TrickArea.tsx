import { useMemo, useEffect, useRef, useState } from 'react'
import { useTranslations } from 'next-intl'
import type {
  Card,
  PhaseSnapshot,
  RoundPublic,
  Seat,
} from '@/lib/game-room/types'
import { PlayingCard, CARD_DIMENSIONS } from './PlayingCard'
import { LastTrickCards } from './LastTrickCards'
import { TrickAreaHeader } from './TrickAreaHeader'
import { cn } from '@/lib/cn'
import { useMediaQuery } from '@/hooks/useMediaQuery'
import { shortenNameForDisplay, getOrientation, getActiveSeat } from './utils'
import {
  isBiddingPhase,
  isTrumpSelectPhase,
  isTrickPhase,
  getLastTrick,
} from './phase-helpers'

// Dimension constants
const CARD_HEIGHT = CARD_DIMENSIONS.md.height
const CARD_WIDTH = CARD_DIMENSIONS.md.width
const LABEL_HEIGHT = 20
const Z_INDEX_BASE = 20

// Header row height (h-7 = 28px)
const HEADER_ROW_HEIGHT = 28
const DIAMOND_LAYOUT_PADDING = 16 // Equal top and bottom padding for >640px diamond layout

// Card transform multipliers
const VERTICAL_OFFSET_MULTIPLIER = 0.1 // 10% of card height for top/bottom positioning
const CARD_OVERLAP_MULTIPLIER = 0.4 // 40% of card width - overlap between cards for centering

// Gap and padding constants
const CARD_LABEL_GAP = 13 // Fixed gap between card and label (absolute value, not scaled)
const MOBILE_TOP_PADDING = 8 // Top padding for <640px
const MOBILE_BOTTOM_PADDING = 6 // Bottom padding for <640px
const MAX_NAME_LENGTH = 8 // Maximum length for shortened player names in trick area

// Diamond layout constants (>640px)
const DIAMOND_VERTICAL_OFFSET = 30 // Vertical offset for top/bottom cards in diamond layout
const DIAMOND_HORIZONTAL_OFFSET = 34 // Horizontal offset for left/right cards in diamond layout
const DIAMOND_CONTAINER_SIZE = 154 // Size of the diamond layout container (h-[154px] w-[154px])
const DIAMOND_MAX_RANDOM_OFFSET = 5 // Maximum random offset (in pixels) for card misalignment

// Linear congruential generator constants for seeded random number generation
const LCG_MULTIPLIER = 1103515245
const LCG_INCREMENT = 12345
const LCG_MODULUS_MASK = 0x7fffffff
const LCG_SEED_MULTIPLIER = 12345
const LCG_SEED_OFFSET = 67890

// Transform constants
const CENTER_TRANSFORM = 'translate(-50%, -50%)' // Base transform for centering absolutely positioned elements

type Orientation = 'top' | 'bottom' | 'left' | 'right'

/**
 * Maps play order to fixed position layout for <640px viewport.
 * Pattern: lower, higher, lower, higher (left to right).
 * First card (playOrder 0) = left (lower), second (1) = top (higher),
 * third (2) = right (lower), fourth (3) = top (higher, continuing pattern).
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
      return 'top' // Changed from 'bottom' to 'top' to continue lower/higher pattern
    default:
      return 'top'
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
  isLargeViewport: boolean,
  isMediumViewport: boolean
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
    // Height = cardHeight + cardOffset + gap + labelHeight + headerRowHeight + 2x topOffset + bottomPadding
    // For 380-640px: reduce by headerRowHeight + 1x topPadding since header row is present
    const verticalOffset =
      scaledDimensions.cardHeight * VERTICAL_OFFSET_MULTIPLIER
    const baseHeight =
      scaledDimensions.cardHeight +
      verticalOffset +
      scaledDimensions.gap +
      scaledDimensions.labelHeight +
      HEADER_ROW_HEIGHT +
      MOBILE_TOP_PADDING * 2 +
      MOBILE_BOTTOM_PADDING

    if (isMediumViewport) {
      // Reduce by header row height + 1x top padding for 380-640px
      return baseHeight - HEADER_ROW_HEIGHT - MOBILE_TOP_PADDING
    }
    return baseHeight
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
  cardScale?: number
  trickDisplayDurationSeconds?: number | null
  onTrickCompletePauseStart?: () => void
  onTrickCompletePauseEnd?: () => void
  onPauseStateChange?: (isPaused: boolean, isViewerLeader: boolean) => void
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
  if (isBiddingPhase(phase)) {
    return t('waitingForBidding', { name: activeName })
  }
  if (isTrumpSelectPhase(phase)) {
    return t('waitingForTrump', { name: activeName })
  }
  if (isTrickPhase(phase)) {
    return t('waitingForLead', { name: activeName })
  }
  return t('waitingForLead')
}

const DEFAULT_TRICK_DISPLAY_DURATION_SECONDS = 2.0

export function TrickArea({
  trickMap: _trickMap,
  getSeatName,
  round,
  phase,
  viewerSeat,
  lastTrick,
  showPreviousRoundPosition,
  className = '',
  cardScale = 1,
  trickDisplayDurationSeconds = null,
  onTrickCompletePauseStart,
  onTrickCompletePauseEnd,
  onPauseStateChange,
}: TrickAreaProps) {
  const t = useTranslations('game.gameRoom.trickArea')

  // Track pause state for completed tricks
  const [isPaused, setIsPaused] = useState(false)
  const [pausedTrick, setPausedTrick] = useState<Array<[Seat, Card]>>([])
  const [pausedTrickNo, setPausedTrickNo] = useState<number | null>(null)
  const pauseTimeoutRef = useRef<NodeJS.Timeout | null>(null)
  const previousTrickNoRef = useRef<number | null>(null)

  // Detect trick completion by tracking trick number changes
  useEffect(() => {
    const currentTrickNo = isTrickPhase(phase) ? phase.data.trick_no : null

    // Initialize previousTrickNoRef on first render if we're in trick phase
    // This ensures we can detect trick completions from the start
    if (currentTrickNo !== null && previousTrickNoRef.current === null) {
      previousTrickNoRef.current = currentTrickNo
    }

    // Detect trick completion: trick number increased
    if (
      currentTrickNo !== null &&
      previousTrickNoRef.current !== null &&
      currentTrickNo > previousTrickNoRef.current &&
      !isPaused &&
      isTrickPhase(phase)
    ) {
      // Trick just completed - use lastTrick data which contains all 4 cards
      const completedTrick = getLastTrick(phase)
      if (completedTrick && completedTrick.length === 4) {
        // Calculate pause duration: null = default, 0 = no pause, other = use value
        const durationSeconds =
          trickDisplayDurationSeconds === null
            ? DEFAULT_TRICK_DISPLAY_DURATION_SECONDS
            : trickDisplayDurationSeconds

        // If duration is 0, skip the pause entirely
        if (durationSeconds === 0) {
          return
        }

        // Clear any existing timeout before setting a new one
        if (pauseTimeoutRef.current) {
          clearTimeout(pauseTimeoutRef.current)
        }

        // Store the trick number for the paused trick to maintain consistent offsets
        setPausedTrickNo(previousTrickNoRef.current)

        setPausedTrick([...completedTrick])
        setIsPaused(true)

        // Check if viewer is the leader of the next trick
        const isViewerLeader =
          isTrickPhase(phase) && phase.data.leader === viewerSeat
        onPauseStateChange?.(true, isViewerLeader)
        onTrickCompletePauseStart?.()

        // Convert seconds to milliseconds for setTimeout
        const durationMs = durationSeconds * 1000

        // Set timeout to end pause
        pauseTimeoutRef.current = setTimeout(() => {
          // Clear pause state
          setIsPaused(false)
          setPausedTrick([])
          setPausedTrickNo(null)
          onPauseStateChange?.(false, false)
          onTrickCompletePauseEnd?.()
          // Clear the timeout ref
          pauseTimeoutRef.current = null
        }, durationMs)
      }
    }

    // If user plays a card during pause, only cancel pause if viewer is the leader
    // (first player) in the next trick. If viewer is not first, they need to wait
    // for the pause to complete so they can see cards already played by others.
    // We detect this by checking if current_trick has a card from viewerSeat
    // (if viewer played, their card will be in current_trick)
    if (
      isPaused &&
      isTrickPhase(phase) &&
      viewerSeat !== null &&
      phase.data.current_trick.length > 0
    ) {
      const lastCard =
        phase.data.current_trick[phase.data.current_trick.length - 1]
      // Only cancel pause if viewer is the leader (first player) of the current trick
      const isViewerLeader = phase.data.leader === viewerSeat
      if (lastCard && lastCard[0] === viewerSeat && isViewerLeader) {
        // Viewer is leader and just played - cancel pause to show their card immediately
        if (pauseTimeoutRef.current) {
          clearTimeout(pauseTimeoutRef.current)
          pauseTimeoutRef.current = null
        }
        setIsPaused(false)
        setPausedTrick([])
        setPausedTrickNo(null)
        onPauseStateChange?.(false, false)
        onTrickCompletePauseEnd?.()
      }
    }

    // If we're no longer in trick phase, cancel any pause
    if (!isTrickPhase(phase) && isPaused) {
      if (pauseTimeoutRef.current) {
        clearTimeout(pauseTimeoutRef.current)
        pauseTimeoutRef.current = null
      }
      setIsPaused(false)
      setPausedTrick([])
      setPausedTrickNo(null)
      onPauseStateChange?.(false, false)
      onTrickCompletePauseEnd?.()
    }

    // Update previous trick number
    previousTrickNoRef.current = currentTrickNo
  }, [
    phase,
    isPaused,
    viewerSeat,
    trickDisplayDurationSeconds,
    onTrickCompletePauseStart,
    onTrickCompletePauseEnd,
    onPauseStateChange,
  ])

  // Separate effect for cleanup to avoid clearing timeout when effect re-runs
  useEffect(() => {
    return () => {
      if (pauseTimeoutRef.current) {
        clearTimeout(pauseTimeoutRef.current)
        pauseTimeoutRef.current = null
      }
    }
  }, [])

  // Use paused trick if in pause state, otherwise use current trick
  // When paused, show the paused trick; when not paused, show current trick (which will be empty after trick completes)
  const displayTrick = useMemo(() => {
    return isPaused && pausedTrick.length > 0
      ? pausedTrick
      : isTrickPhase(phase)
        ? phase.data.current_trick
        : []
  }, [isPaused, pausedTrick, phase])

  const displayTrickMap = useMemo(() => {
    // Only use paused trick if we're actually paused
    if (isPaused && pausedTrick.length > 0) {
      return new Map(pausedTrick)
    }
    // Otherwise use current trick (empty after trick completes)
    return new Map(displayTrick)
  }, [displayTrick, isPaused, pausedTrick])

  const orderedCards = useMemo(() => {
    // Create a map of seat to play order index from display trick
    const playOrderMap = new Map<Seat, number>()
    displayTrick.forEach(([seat], playIndex) => {
      playOrderMap.set(seat, playIndex)
    })

    const cards = Array.from(displayTrickMap.entries()).map(([seat, card]) => {
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
  }, [displayTrickMap, displayTrick, getSeatName])

  // Calculate total bids
  const totalBids = useMemo(() => {
    if (!round) return 0
    return round.bids.reduce((sum: number, bid) => sum + (bid ?? 0), 0)
  }, [round])

  // Check viewport sizes for responsive adjustments
  const isLargeViewport = useMediaQuery('(min-width: 640px)')
  const isMediumViewport = useMediaQuery('(min-width: 380px)')

  // Generate random offsets for diamond layout cards based on trick number
  // Offsets change when trick number changes to create subtle misalignment
  // When paused, use the trick number from the paused trick to maintain consistent offsets
  const trickNo =
    isPaused && pausedTrickNo !== null
      ? pausedTrickNo
      : isTrickPhase(phase)
        ? phase.data.trick_no
        : 0
  const diamondOffsets = useMemo(() => {
    // Seed random generator with trick number for consistency
    // Simple seeded random function (linear congruential generator)
    // Generate seeds for each random value call without mutation
    const baseSeed = trickNo * LCG_SEED_MULTIPLIER + LCG_SEED_OFFSET

    const getRandom = (callIndex: number): number => {
      // Calculate seed for this specific call index
      let seed = baseSeed
      for (let i = 0; i < callIndex; i++) {
        seed = (seed * LCG_MULTIPLIER + LCG_INCREMENT) & LCG_MODULUS_MASK
      }
      const nextSeed =
        (seed * LCG_MULTIPLIER + LCG_INCREMENT) & LCG_MODULUS_MASK
      return nextSeed / LCG_MODULUS_MASK
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
  const isBetweenRounds = isBiddingPhase(phase) || isTrumpSelectPhase(phase)
  const hasCards = orderedCards.length > 0
  // Calculate container height - always based on card dimensions to prevent layout jumps
  const calculatedHeight = calculateContainerHeight(
    scaledDimensions,
    isLargeViewport,
    isMediumViewport
  )

  // Calculate padding values
  const paddingTop = isLargeViewport
    ? DIAMOND_LAYOUT_PADDING
    : MOBILE_TOP_PADDING
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
        'oldtime-trick-bg relative flex items-center justify-center rounded-[32px] border border-border/60 bg-overlay/25 px-4 text-center text-sm text-muted-foreground shadow-elevated backdrop-blur',
        className
      )}
      style={{
        // With border-box, height includes padding, so add padding to calculatedHeight
        height: `${calculatedHeight + paddingTop + paddingBottom}px`,
        paddingTop: `${paddingTop}px`,
        paddingBottom: `${paddingBottom}px`,
      }}
      role="region"
      aria-label={t('ariaLabel')}
    >
      {/* Header Row - only visible <1024px, positioned absolutely at top */}
      {round && (
        <div
          className="pointer-events-none absolute left-4 right-4 top-0 z-10 lg:hidden"
          style={{
            top: `${paddingTop}px`,
          }}
        >
          <div className="pointer-events-auto">
            <TrickAreaHeader
              trump={round.trump}
              totalBids={totalBids}
              handSize={round.hand_size ?? 0}
            />
          </div>
        </div>
      )}

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
          <span className="text-sm font-medium text-muted-foreground">
            {getWaitingMessage(phase, getSeatName, t)}
          </span>
          {isTrickPhase(phase) ? (
            <span className="text-xs text-muted-foreground">
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
            {(isTrickPhase(phase) || isPaused) &&
              displayTrick.length > 0 &&
              displayTrick.map(([seat, card], playOrder) => {
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
                    className="absolute left-1/2 top-1/2"
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
            // is 2 * paddingTop + headerRowHeight (for <640px only)
            // For 380-640px: reduce by headerRowHeight + 1x topPadding since header row is present
            // After translateY(-verticalOffset), center = centerTop - verticalOffset
            // Top edge = center - (cardHeight + gap + labelHeight) / 2
            // For top edge = visibleGap: centerTop - verticalOffset - (cardHeight + gap + labelHeight) / 2 = visibleGap
            // So: centerTop = visibleGap + verticalOffset + (cardHeight + gap + labelHeight) / 2
            const cardVerticalOffset =
              scaledDimensions.cardHeight * VERTICAL_OFFSET_MULTIPLIER
            const totalElementHeight =
              scaledDimensions.cardHeight +
              scaledDimensions.gap +
              scaledDimensions.labelHeight
            let visibleGap = paddingTop * 2 + HEADER_ROW_HEIGHT
            if (isMediumViewport) {
              // Reduce by header row height + 1x top padding for 380-640px
              visibleGap = visibleGap - HEADER_ROW_HEIGHT - MOBILE_TOP_PADDING
            }
            const centerTop =
              visibleGap + cardVerticalOffset + totalElementHeight / 2

            // Calculate final horizontal offsets once for all cards
            // Pattern: left-to-right, lower-then-higher-then-lower, centered accounting for overlap
            // All calculations start from container center (50% across)
            const cardWidth = scaledDimensions.cardWidth
            const cardHeight = scaledDimensions.cardHeight
            const cardHalfWidth = cardWidth / 2
            const cardVerticalOffsetValue =
              cardHeight * VERTICAL_OFFSET_MULTIPLIER

            // Overlap between cards (proportional to card width)
            const overlap = CARD_OVERLAP_MULTIPLIER * cardWidth
            const overlapHalf = overlap / 2
            // Adjustment for overlap: move by (1 - overlapMultiplier) to account for actual overlap
            const overlapAdjustment = (1 - CARD_OVERLAP_MULTIPLIER) * cardWidth

            // Calculate horizontal offsets from container center
            // Cards 2 and 3: symmetric inner positions
            // Cards 1 and 4: outer positions adjusted for overlap
            const finalHorizontalOffsets: Record<string, number> = {
              'left-0': -cardHalfWidth + overlapHalf - overlapAdjustment, // Card 1: -0.5 + 0.2 - 0.6 = -0.9 * cardWidth
              'top-1': -(cardHalfWidth - overlapHalf), // Card 2: -(0.5 - 0.2) = -0.3 * cardWidth
              'right-2': cardHalfWidth - overlapHalf, // Card 3: (0.5 - 0.2) = 0.3 * cardWidth
              'top-3': cardHalfWidth - overlapHalf + overlapAdjustment, // Card 4: 0.5 - 0.2 + 0.6 = 0.9 * cardWidth
            }

            // Pre-calculated vertical offsets (only for top positions)
            const finalVerticalOffsets: Record<string, number> = {
              'top-1': -cardVerticalOffsetValue,
              'top-3': -cardVerticalOffsetValue * 1.2,
            }

            return orderedCards.map(
              ({ seat, card, label, fixedPosition, playOrder }) => {
                // Get pre-calculated transforms - all positions are defined, so no fallback needed
                const key = `${fixedPosition}-${playOrder}`
                const horizontalOffsetValue = finalHorizontalOffsets[key]
                const verticalOffsetValue = finalVerticalOffsets[key] ?? 0

                const transforms: string[] = []
                if (horizontalOffsetValue !== 0) {
                  transforms.push(`translateX(${horizontalOffsetValue}px)`)
                }
                if (verticalOffsetValue !== 0) {
                  transforms.push(`translateY(${verticalOffsetValue}px)`)
                }
                if (safeCardScale !== 1) {
                  transforms.push(`scale(${safeCardScale})`)
                }

                const positionTransform =
                  transforms.length > 0 ? transforms.join(' ') : 'none'
                const combinedTransform =
                  positionTransform === 'none'
                    ? CENTER_TRANSFORM
                    : `${CENTER_TRANSFORM} ${positionTransform}`

                return (
                  <div
                    key={seat}
                    className="absolute left-1/2 flex flex-col items-center"
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
