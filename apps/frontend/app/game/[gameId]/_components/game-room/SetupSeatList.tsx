type SetupSeat = {
  seat: number
  seatNumber: number
  name: string
  isAi: boolean
  isReady: boolean
  isOccupied: boolean
  isViewer: boolean
}

interface SetupSeatListProps {
  seats: SetupSeat[]
}

export function SetupSeatList({ seats }: SetupSeatListProps) {
  return (
    <div className="rounded-3xl border border-white/10 bg-surface/80 p-5 shadow-[0_35px_100px_rgba(0,0,0,0.35)] backdrop-blur">
      <header className="mb-4 flex flex-col gap-1">
        <p className="text-xs font-semibold uppercase tracking-[0.4em] text-subtle">
          Seating
        </p>
        <h2 className="text-2xl font-semibold text-foreground">
          Who is sitting where
        </h2>
        <p className="text-sm text-muted">
          Each seat shows whether it is filled by a human player, an AI seat, or
          still open.
        </p>
      </header>

      <ul className="space-y-3">
        {seats.map((seat) => {
          const statusLabel = seat.isAi
            ? 'AI player'
            : seat.isOccupied
              ? 'Human player'
              : 'Open seat'

          return (
            <li
              key={seat.seat}
              className={`rounded-2xl border px-4 py-3 transition ${
                seat.isViewer
                  ? 'border-primary/60 bg-primary/5'
                  : 'border-border/60 bg-surface/70'
              }`}
            >
              <div className="flex flex-wrap items-center justify-between gap-2">
                <div>
                  <p className="text-[11px] font-semibold uppercase tracking-[0.4em] text-subtle">
                    Seat {seat.seatNumber}
                  </p>
                  <p className="text-base font-semibold text-foreground">
                    {seat.name}
                  </p>
                </div>
                <span
                  className={`rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-wide ${
                    seat.isReady
                      ? 'bg-success/20 text-success-contrast'
                      : 'bg-border/20 text-subtle'
                  }`}
                >
                  {seat.isReady ? 'Ready' : 'Not ready'}
                </span>
              </div>

              <div className="mt-3 flex flex-wrap gap-2 text-[11px] uppercase tracking-wide text-subtle">
                <span className="rounded-full border border-border/40 px-3 py-1 text-xs">
                  {statusLabel}
                </span>
                {seat.isViewer ? (
                  <span className="rounded-full border border-primary/50 px-3 py-1 text-xs text-primary">
                    Your seat
                  </span>
                ) : null}
              </div>
            </li>
          )
        })}
      </ul>
    </div>
  )
}
