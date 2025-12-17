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
          <div className="relative flex max-w-[280px] flex-wrap items-center justify-center gap-0 overflow-visible px-2">
            {orderedCards.map(({ seat, card, label, orientation }, index) => {
              const offsetClass =
                orientation === 'top'
                  ? '-translate-y-3'
                  : orientation === 'bottom'
                    ? 'translate-y-3'
                    : orientation === 'left'
                      ? '-translate-x-2'
                      : 'translate-x-2'
              return (
                <div
                  key={seat}
                  className={cn(
                    'relative flex flex-col items-center gap-2 transition-all duration-300',
                    index > 0 ? '-ml-8' : '',
                    offsetClass
                  )}
                  style={{ zIndex: 20 + index }}
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
