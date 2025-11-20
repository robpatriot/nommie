import type { Card, Seat } from '@/lib/game-room/types'
import { PlayingCard } from './PlayingCard'
import { getOrientation, shortenNameForDisplay } from './utils'
import { cn } from '@/lib/cn'

interface LastTrickProps {
  lastTrick: Array<[Seat, Card]> | null
  getSeatName: (seat: Seat) => string
  viewerSeat: Seat
}

export function LastTrick({
  lastTrick,
  getSeatName,
  viewerSeat,
}: LastTrickProps) {
  if (!lastTrick || lastTrick.length === 0) {
    return (
      <section className="flex w-full flex-col gap-4 rounded-3xl border border-white/10 bg-surface/80 p-5 text-sm text-muted shadow-[0_25px_80px_rgba(0,0,0,0.35)] backdrop-blur">
        <header className="flex items-center justify-between">
          <h2 className="text-sm font-semibold uppercase tracking-[0.4em] text-subtle">
            Last trick
          </h2>
        </header>
        <p className="text-xs text-muted">No trick completed yet.</p>
      </section>
    )
  }

  const cards = lastTrick.map(([seat, card]) => ({
    seat,
    card,
    label: getSeatName(seat),
    orientation: getOrientation(viewerSeat, seat),
  }))

  const orientationOrder: Array<'bottom' | 'right' | 'top' | 'left'> = [
    'left',
    'top',
    'right',
    'bottom',
  ]
  const orderedCards = cards
    .slice()
    .sort(
      (a, b) =>
        orientationOrder.indexOf(a.orientation) -
        orientationOrder.indexOf(b.orientation)
    )

  return (
    <section className="flex w-full flex-col gap-4 rounded-3xl border border-white/10 bg-surface/80 p-5 text-sm text-muted shadow-[0_25px_80px_rgba(0,0,0,0.35)] backdrop-blur">
      <header className="flex items-center justify-between">
        <h2 className="text-sm font-semibold uppercase tracking-[0.4em] text-subtle">
          Last trick
        </h2>
      </header>
      <div className="relative flex min-h-[200px] items-center justify-center overflow-visible py-8">
        {/* Bounding box for cards - larger to accommodate names */}
        <div className="relative mx-auto h-[154px] w-[154px]">
          {/* Cards container - using absolute positioning for diamond layout */}
          {orderedCards.map(({ seat, card, orientation }, index) => {
            // Calculate position offsets for diamond shape
            // All cards start centered, then offset based on orientation
            // Using inline styles to combine transforms properly
            const getTransform = () => {
              const base = 'translate(-50%, -50%)'
              switch (orientation) {
                case 'top':
                  return `${base} translateY(-30px)` // Move up (reduced)
                case 'bottom':
                  return `${base} translateY(30px)` // Move down (reduced, same x as top)
                case 'left':
                  return `${base} translateX(-34px)` // Move left
                case 'right':
                  return `${base} translateX(34px)` // Move right (same y as left)
                default:
                  return base
              }
            }

            return (
              <div
                key={seat}
                className="absolute left-1/2 top-1/2 transition-all duration-300"
                style={{
                  zIndex: 20 + index,
                  transform: getTransform(),
                }}
              >
                <div className="relative">
                  <PlayingCard card={card} size="sm" />
                </div>
              </div>
            )
          })}

          {/* Names positioned relative to bounding box */}
          {orderedCards.map(({ seat, label, orientation }) => {
            // Determine max length based on orientation
            // Top/Bottom have more horizontal space, Left/Right have less
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
                key={seat}
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
    </section>
  )
}
