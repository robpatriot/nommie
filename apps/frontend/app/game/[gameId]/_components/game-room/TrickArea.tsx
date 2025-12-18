import { useMemo } from 'react'
import type { PhaseSnapshot, RoundPublic, Seat } from '@/lib/game-room/types'
import type { Card } from '@/lib/game-room/types'
import { getOrientation, ORIENTATION_ORDER_TRICK } from './utils'
import { PlayingCard } from './PlayingCard'
import { LastTrickCards } from './LastTrickCards'
import { SyncButton } from './SyncButton'
import { cn } from '@/lib/cn'
import { useMediaQuery } from '@/hooks/useMediaQuery'

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

  // Show last trick during bidding/trump selection (previous round's final trick)
  // when there's no current trick being played
  const isBetweenRounds =
    phase.phase === 'Bidding' || phase.phase === 'TrumpSelect'
  const showLastTrick =
    (showPreviousRoundPosition ??
      (isBetweenRounds && orderedCards.length === 0)) &&
    lastTrick &&
    lastTrick.length > 0

  // Check if viewport is > 640px for responsive top padding
  const isLargeViewport = useMediaQuery('(min-width: 640px)')

  // Dimension constants
  const CARD_HEIGHT = 112
  const LABEL_HEIGHT = 20
  const TOP_PADDING_BASE = 60
  const BOTTOM_PADDING = 16
  const BASE_GAP = 8
  const BASE_MARGIN = 32
  const OVERLAP_MULTIPLIER = 3
  const GAP_MULTIPLIER = 2
  const PADDING_INCREASE_MULTIPLIER = 0.75

  // Orientation offset constants
  const ORIENTATION_OFFSETS = {
    top: -12,
    bottom: 12,
    left: -8,
    right: 8,
  } as const

  // Calculate responsive top padding
  const TOP_PADDING = isLargeViewport ? TOP_PADDING_BASE / 2 : TOP_PADDING_BASE

  // Scale dimensions with cardScale proportionally
  const scaledCardHeight = CARD_HEIGHT * cardScale
  const scaledLabelHeight = LABEL_HEIGHT * cardScale
  const scaledBottomPadding = BOTTOM_PADDING * cardScale

  // Top padding increases as cards scale down to maintain space for sync button
  const scaleDifference = 1 - cardScale
  const paddingIncrease =
    TOP_PADDING * scaleDifference * PADDING_INCREASE_MULTIPLIER
  const scaledTopPadding = TOP_PADDING + paddingIncrease

  // Gap increases as cards scale down (for visual spacing between card and label)
  const scaledGap =
    cardScale < 1 ? (BASE_GAP / cardScale) * GAP_MULTIPLIER : BASE_GAP

  // Overlap adjustment - move cards closer together as they scale down
  const overlapAdjustment =
    cardScale < 1 ? BASE_MARGIN * (1 - cardScale) * OVERLAP_MULTIPLIER : 0

  const hasCards = orderedCards.length > 0
  const calculatedHeight = hasCards
    ? scaledCardHeight +
      scaledLabelHeight +
      scaledGap +
      scaledTopPadding +
      scaledBottomPadding
    : 100

  return (
    <div
      className={cn(
        'relative flex items-center justify-center rounded-[32px] border border-white/10 bg-black/25 px-4 text-center text-sm text-muted shadow-[0_35px_90px_rgba(0,0,0,0.4)] backdrop-blur',
        className
      )}
      style={{
        height: hasCards ? `${calculatedHeight}px` : 'auto',
        minHeight: hasCards ? `${calculatedHeight}px` : 100,
        paddingTop: `${scaledTopPadding}px`,
        paddingBottom: `${scaledBottomPadding}px`,
      }}
    >
      {onRefresh ? (
        <div className="pointer-events-auto absolute right-4 top-4 z-10 sm:hidden">
          <SyncButton onRefresh={onRefresh} isRefreshing={isRefreshing} />
        </div>
      ) : null}
      {showLastTrick ? (
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

            // Calculate transform based on orientation and overlap adjustment
            const offsetTransforms: string[] = []
            if (orientation === 'top') {
              offsetTransforms.push(`translateY(${ORIENTATION_OFFSETS.top}px)`)
            } else if (orientation === 'bottom') {
              offsetTransforms.push(
                `translateY(${ORIENTATION_OFFSETS.bottom}px)`
              )
            } else if (orientation === 'left') {
              // Left card moves right (more positive) to get closer to center
              const totalX = ORIENTATION_OFFSETS.left + overlapAdjustment
              offsetTransforms.push(`translateX(${totalX}px)`)
            } else {
              // Right card moves left (more negative) to get closer to center
              const totalX = ORIENTATION_OFFSETS.right - overlapAdjustment
              offsetTransforms.push(`translateX(${totalX}px)`)
            }

            // Apply scale transform (applied last)
            if (cardScale !== 1) {
              offsetTransforms.push(`scale(${cardScale})`)
            }

            const combinedTransform = offsetTransforms.join(' ')

            return (
              <div
                key={seat}
                className="relative flex flex-col items-center transition-all duration-300"
                style={{
                  zIndex: 20 + index,
                  transform: combinedTransform,
                  transformOrigin: 'center center',
                  marginLeft: baseMarginLeft,
                  gap: `${scaledGap}px`,
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
