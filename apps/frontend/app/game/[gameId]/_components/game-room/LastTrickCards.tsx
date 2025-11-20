import type { Card, Seat } from '@/lib/game-room/types'
import { PlayingCard } from './PlayingCard'
import { getOrientation, shortenNameForDisplay } from './utils'
import { cn } from '@/lib/cn'

interface LastTrickCardsProps {
  lastTrick: Array<[Seat, Card]>
  getSeatName: (seat: Seat) => string
  viewerSeat: Seat
  showNames?: boolean
}

/**
 * Shared component for rendering last trick cards in diamond layout.
 * Used by both LastTrick panel and TrickArea.
 */
export function LastTrickCards({
  lastTrick,
  getSeatName,
  viewerSeat,
  showNames = true,
}: LastTrickCardsProps) {
  return (
    <div className="relative flex min-h-[200px] items-center justify-center overflow-visible py-8">
      {/* Bounding box for cards - larger to accommodate names */}
      <div className="relative mx-auto h-[154px] w-[154px]">
        {/* Cards container - using absolute positioning for diamond layout */}
        {/* Cards are displayed in play order, with later cards layered on top */}
        {lastTrick.map(([seat, card], playOrder) => {
          const orientation = getOrientation(viewerSeat, seat)

          // Calculate position offsets for diamond shape
          const getTransform = () => {
            const base = 'translate(-50%, -50%)'
            switch (orientation) {
              case 'top':
                return `${base} translateY(-30px)`
              case 'bottom':
                return `${base} translateY(30px)`
              case 'left':
                return `${base} translateX(-34px)`
              case 'right':
                return `${base} translateX(34px)`
              default:
                return base
            }
          }

          return (
            <div
              key={`${seat}-${playOrder}`}
              className="absolute left-1/2 top-1/2 transition-all duration-300"
              style={{
                zIndex: 20 + playOrder,
                transform: getTransform(),
              }}
            >
              <div className="relative">
                <PlayingCard card={card} size="sm" />
              </div>
            </div>
          )
        })}

        {/* Names positioned relative to bounding box - only render if showNames is true */}
        {showNames &&
          lastTrick.map(([seat], index) => {
            const orientation = getOrientation(viewerSeat, seat)
            const label = getSeatName(seat)
            const maxLength =
              orientation === 'top' || orientation === 'bottom' ? 18 : 8
            const shortenedLabel = shortenNameForDisplay(label, maxLength)

            const namePositionClass =
              orientation === 'top'
                ? 'absolute -top-6 left-1/2 -translate-x-1/2 whitespace-nowrap'
                : orientation === 'bottom'
                  ? 'absolute -bottom-6 left-1/2 -translate-x-1/2 whitespace-nowrap'
                  : orientation === 'left'
                    ? 'absolute left-0 top-1/2 -translate-y-1/2 -translate-x-full -mr-2 whitespace-nowrap text-right'
                    : 'absolute right-0 top-1/2 -translate-y-1/2 translate-x-full ml-2 whitespace-nowrap text-left'

            return (
              <span
                key={`name-${seat}-${index}`}
                className={cn(
                  'text-[10px] font-semibold uppercase tracking-[0.3em] text-foreground',
                  namePositionClass
                )}
              >
                {shortenedLabel}
              </span>
            )
          })}
      </div>
    </div>
  )
}
