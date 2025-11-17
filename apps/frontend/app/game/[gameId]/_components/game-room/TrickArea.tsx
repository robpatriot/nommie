import type { PhaseSnapshot, RoundPublic, Seat } from '@/lib/game-room/types'
import type { Card } from '@/lib/game-room/types'
import { getOrientation } from './utils'

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

  return (
    <div
      className={`flex flex-col items-center justify-center gap-4 rounded-3xl border border-white/10 bg-surface/80 p-6 text-center shadow-[0_20px_60px_rgba(0,0,0,0.35)] backdrop-blur ${className}`}
    >
      <p className="text-sm font-semibold uppercase tracking-[0.4em] text-subtle">
        Current trick
      </p>
      <div className="flex flex-wrap items-center justify-center gap-6">
        {cards.length === 0 ? (
          <span className="text-sm text-muted">Waiting for lead…</span>
        ) : (
          cards.map(({ seat, card, label, orientation }) => (
            <div key={seat} className="flex flex-col items-center gap-2">
              <span className="text-xs uppercase tracking-wide text-subtle">
                {label}
              </span>
              <span className="rounded-2xl bg-surface px-4 py-2 text-xl font-semibold tracking-widest text-foreground">
                {card}
              </span>
              <span className="text-[10px] uppercase text-subtle">
                {orientation}
              </span>
            </div>
          ))
        )}
      </div>
      {phase.phase === 'Trick' ? (
        <p className="text-xs text-muted">
          Leader: {getSeatName(phase.data.leader)} — Trick {phase.data.trick_no}{' '}
          of {round?.hand_size ?? '?'}
        </p>
      ) : null}
    </div>
  )
}
