import { useMemo } from 'react'
import type { PhaseSnapshot, RoundPublic, Seat } from '@/lib/game-room/types'
import type { Card } from '@/lib/game-room/types'
import { getOrientation, ORIENTATION_ORDER_TRICK } from './utils'
import { PlayingCard } from './PlayingCard'
import { LastTrickCards } from './LastTrickCards'
import { SyncButton } from './SyncButton'
import { cn } from '@/lib/cn'

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

  // Calculate minimum height based on number of cards played
  // Card height (112px) + label (~20px) + gap (8px) + padding (60px top + 16px bottom)
  const CARD_HEIGHT = 112
  const LABEL_HEIGHT = 20
  const GAP = 8
  const VERTICAL_PADDING = 76 // pt-[60px] (60px) + pb-4 (16px)

  const hasCards = orderedCards.length > 0
  const minHeight = hasCards
    ? CARD_HEIGHT + LABEL_HEIGHT + GAP + VERTICAL_PADDING
    : 100 // Smaller when empty

  return (
    <div
      className={cn(
        'relative flex items-center justify-center rounded-[32px] border border-white/10 bg-black/25 px-4 pt-[60px] pb-4 text-center text-sm text-muted shadow-[0_35px_90px_rgba(0,0,0,0.4)] backdrop-blur',
        className
      )}
      style={{ minHeight }}
    >
      {onRefresh ? (
        <div className="pointer-events-auto absolute right-4 top-4 z-10 sm:hidden">
          <SyncButton onRefresh={onRefresh} isRefreshing={isRefreshing} />
        </div>
      ) : null}
      {showLastTrick ? (
        <>
          <div className="flex w-full flex-col gap-4">
            <LastTrickCards
              lastTrick={lastTrick}
              getSeatName={getSeatName}
              viewerSeat={viewerSeat}
              showNames={false}
            />
          </div>
        </>
      ) : orderedCards.length === 0 ? (
        <>
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
        </>
      ) : (
        <>
          {/* Cards positioned inside a bounded fan area */}
          <div className="relative flex items-center justify-center gap-0 overflow-visible px-2">
            {orderedCards.map(({ seat, card, label, orientation }, index) => {
              // Base margin for overlap
              const baseMargin = 32
              const baseMarginLeft = index > 0 ? -baseMargin : 0

              // Calculate overlap adjustment - move cards closer together as they scale down
              // Apply to all cards (not just index > 0) so they all move toward center
              // Make it much more pronounced to see the effect
              const overlapAdjustment =
                cardScale < 1
                  ? baseMargin * (1 - cardScale) * 3 // Multiply by 3 to make it very pronounced
                  : 0

              // Calculate offset transforms based on orientation
              const offsetTransforms: string[] = []
              if (orientation === 'top') {
                offsetTransforms.push('translateY(-12px)')
              } else if (orientation === 'bottom') {
                offsetTransforms.push('translateY(12px)')
              } else if (orientation === 'left') {
                // Combine orientation offset with overlap adjustment
                // Left card should move RIGHT (more positive) to get closer to center
                const totalX = -8 + overlapAdjustment
                offsetTransforms.push(`translateX(${totalX}px)`)
              } else {
                // Combine orientation offset with overlap adjustment
                // Right card should move LEFT (more negative) to get closer to center
                const totalX = 8 - overlapAdjustment
                offsetTransforms.push(`translateX(${totalX}px)`)
              }

              // Apply scale transform from prop (applied last)
              if (cardScale !== 1) {
                offsetTransforms.push(`scale(${cardScale})`)
              }

              const combinedTransform =
                offsetTransforms.length > 0
                  ? offsetTransforms.join(' ')
                  : undefined

              // Increase gap between card and label as cards scale down (2x more pronounced)
              // At scale 1.0: gap = 8px (gap-2)
              // At scale 0.8: gap = 20px (8 / 0.8 * 2)
              // At scale 0.75: gap = 21.33px (8 / 0.75 * 2)
              const baseGap = 8 // gap-2 = 8px
              const scaledGap =
                cardScale < 1 ? (baseGap / cardScale) * 2 : baseGap

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
        </>
      )}
    </div>
  )
}
