import type { PhaseSnapshot, RoundPublic, Seat } from '@/lib/game-room/types'
import type { Card } from '@/lib/game-room/types'
import { getOrientation } from './utils'

interface TrickAreaProps {
  trickMap: Map<Seat, Card>
  getSeatName: (seat: Seat) => string
  round: RoundPublic | null
  phase: PhaseSnapshot
  viewerSeat: Seat
}

export function TrickArea({
  trickMap,
  getSeatName,
  round,
  phase,
  viewerSeat,
}: TrickAreaProps) {
  const cards = Array.from(trickMap.entries()).map(([seat, card]) => ({
    seat,
    card,
    label: getSeatName(seat),
    orientation: getOrientation(viewerSeat, seat),
  }))

  return (
    <div className="col-start-2 row-start-2 flex h-64 flex-col items-center justify-center gap-4 rounded-2xl border border-border bg-surface/70 p-6">
      <p className="text-sm uppercase tracking-wide text-subtle">
        Current Trick
      </p>
      <div className="flex flex-wrap items-center justify-center gap-6">
        {cards.length === 0 ? (
          <span className="text-sm text-subtle">Waiting for lead…</span>
        ) : (
          cards.map(({ seat, card, label, orientation }) => (
            <div key={seat} className="flex flex-col items-center gap-2">
              <span className="text-xs uppercase tracking-wide text-subtle">
                {label}
              </span>
              <span className="rounded-xl bg-surface px-3 py-2 text-lg font-semibold tracking-wider text-foreground">
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
        <p className="text-xs text-subtle">
          Leader: {getSeatName(phase.data.leader)} — Trick {phase.data.trick_no}{' '}
          of {round?.hand_size ?? '?'}
        </p>
      ) : null}
    </div>
  )
}
