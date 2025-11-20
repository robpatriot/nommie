import type { PhaseSnapshot, RoundPublic, Seat } from '@/lib/game-room/types'
import type { Card } from '@/lib/game-room/types'
import { getOrientation } from './utils'
import { PlayingCard } from './PlayingCard'
import { cn } from '@/lib/cn'

interface TrickAreaProps {
  trickMap: Map<Seat, Card>
  getSeatName: (seat: Seat) => string
  round: RoundPublic | null
  phase: PhaseSnapshot
  viewerSeat: Seat
  className?: string
}

export function TrickArea({
  trickMap,
  getSeatName,
  round,
  phase,
  viewerSeat,
  className = '',
}: TrickAreaProps) {
  const cards = Array.from(trickMap.entries()).map(([seat, card]) => ({
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
    <div
      className={cn(
        'relative flex h-full min-h-[280px] items-center justify-center rounded-[32px] border border-white/10 bg-black/25 p-8 text-center text-sm text-muted shadow-[0_35px_90px_rgba(0,0,0,0.4)] backdrop-blur',
        className
      )}
    >
      {cards.length === 0 ? (
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
                  <div className="relative">
                    <PlayingCard card={card} size="md" />
                    {/* Subtle glow for active player */}
                    {phase.phase === 'Trick' && phase.data.to_act === seat ? (
                      <div className="absolute inset-0 -z-10 rounded-[1.35rem] bg-success/20 blur-xl" />
                    ) : null}
                  </div>
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
