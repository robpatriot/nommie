import type { SeatSummary } from './utils'

export function SeatCard({ summary }: { summary: SeatSummary }) {
  const {
    orientation,
    name,
    score,
    isViewer,
    tricksWon,
    currentCard,
    bid,
    isActive,
  } = summary

  const positionStyles: Record<SeatSummary['orientation'], string> = {
    top: 'col-start-2 row-start-1 justify-self-center',
    left: 'col-start-1 row-start-2 justify-self-start',
    right: 'col-start-3 row-start-2 justify-self-end',
    bottom: 'col-start-2 row-start-3 justify-self-center',
  }

  return (
    <div
      className={`flex w-full max-w-[220px] flex-col gap-2 rounded-xl border border-border bg-surface/70 p-3 text-center shadow-elevated ${
        isActive
          ? 'ring-2 ring-success ring-offset-2 ring-offset-background'
          : ''
      } ${positionStyles[orientation]}`}
    >
      <div className="flex flex-col gap-1">
        <span className="text-xs uppercase tracking-wide text-subtle">
          {orientation === 'bottom' ? 'You' : 'Player'}
        </span>
        <span className="text-lg font-semibold text-foreground">{name}</span>
        <span className="text-xs text-subtle">Score {score}</span>
      </div>
      <div className="flex items-center justify-center gap-3 text-xs text-muted">
        {typeof tricksWon === 'number' ? (
          <span className="rounded-full bg-surface px-2 py-1 font-medium text-foreground">
            Tricks {tricksWon}
          </span>
        ) : null}
        {bid !== undefined ? (
          <span className="rounded-full border border-border px-2 py-1 font-medium">
            Bid {bid ?? '—'}
          </span>
        ) : null}
        {currentCard ? (
          <span className="rounded-md bg-surface px-2 py-1 font-semibold tracking-wide text-foreground">
            {currentCard}
          </span>
        ) : null}
      </div>
      {isViewer ? (
        <span className="self-center rounded-full bg-success/15 px-3 py-1 text-xs font-semibold text-success-foreground">
          You
        </span>
      ) : null}
    </div>
  )
}
