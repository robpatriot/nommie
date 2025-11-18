import type { SeatSummary } from './utils'

type SeatCardProps = {
  summary: SeatSummary
  variant?: 'table' | 'list'
  className?: string
}

export function SeatCard({
  summary,
  variant = 'table',
  className = '',
}: SeatCardProps) {
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
    top: 'lg:col-start-2 lg:row-start-1 lg:justify-self-center',
    left: 'lg:col-start-1 lg:row-start-2 lg:justify-self-start',
    right: 'lg:col-start-3 lg:row-start-2 lg:justify-self-end',
    bottom: 'lg:col-start-2 lg:row-start-3 lg:justify-self-center',
  }

  const baseClasses =
    variant === 'table'
      ? 'flex max-w-[230px] flex-col items-center text-center'
      : 'flex w-full flex-col gap-2 sm:flex-row sm:items-center sm:justify-between text-left'

  const badge =
    orientation === 'bottom'
      ? isViewer
        ? 'You'
        : 'South seat'
      : `${orientation.charAt(0).toUpperCase()}${orientation.slice(1)} seat`

  return (
    <div
      className={`rounded-3xl border border-white/10 bg-surface/80 p-4 shadow-[0_12px_45px_rgba(0,0,0,0.35)] backdrop-blur transition ${
        isActive
          ? 'ring-2 ring-success/80 ring-offset-4 ring-offset-surface'
          : ''
      } ${variant === 'table' ? positionStyles[orientation] : ''} ${baseClasses} ${className}`}
    >
      <div className="flex flex-col gap-1">
        <span className="text-[11px] font-semibold uppercase tracking-[0.4em] text-subtle">
          {badge}
        </span>
        <span className="text-xl font-semibold text-foreground">{name}</span>
        <span className="text-xs text-muted">Score {score}</span>
      </div>

      <div
        className={`mt-3 flex flex-wrap items-center justify-center gap-2 text-xs text-muted ${
          variant === 'list' ? 'sm:justify-end' : ''
        }`}
      >
        {typeof tricksWon === 'number' ? (
          <span className="rounded-full bg-surface px-3 py-1 font-semibold text-foreground">
            Tricks {tricksWon}
          </span>
        ) : null}
        {bid !== undefined ? (
          <span className="rounded-full border border-border/70 px-3 py-1 font-semibold">
            Bid {bid ?? '—'}
          </span>
        ) : null}
        {currentCard ? (
          <span className="rounded-xl bg-surface px-3 py-1 text-sm font-semibold tracking-wide text-foreground">
            {currentCard}
          </span>
        ) : null}
      </div>

      {isViewer ? (
        <span className="mt-3 self-center rounded-full bg-success/20 px-3 py-1 text-xs font-semibold uppercase tracking-wide text-success-contrast">
          You
        </span>
      ) : null}
    </div>
  )
}
