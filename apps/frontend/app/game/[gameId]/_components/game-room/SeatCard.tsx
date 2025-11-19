import { cn } from '@/lib/cn'
import { PlayingCard } from './PlayingCard'
import type { SeatSummary } from './utils'

type SeatCardProps = {
  summary: SeatSummary
  variant?: 'table' | 'list'
  className?: string
  showBid?: boolean
}

export function SeatCard({
  summary,
  variant = 'table',
  className = '',
  showBid = true,
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

  const badge =
    orientation === 'bottom'
      ? isViewer
        ? 'You'
        : 'South seat'
      : `${orientation.charAt(0).toUpperCase()}${orientation.slice(1)} seat`

  const baseClasses =
    variant === 'table'
      ? 'max-w-[240px] items-center text-center'
      : 'max-w-none text-left sm:flex-row sm:items-center sm:justify-between'

  return (
    <div
      className={cn(
        'flex w-full flex-col gap-3 rounded-3xl border border-white/10 bg-surface/80 p-4 text-sm text-muted shadow-[0_18px_65px_rgba(0,0,0,0.35)] backdrop-blur transition',
        baseClasses,
        isActive
          ? 'ring-2 ring-success/80 ring-offset-4 ring-offset-surface'
          : '',
        variant === 'table' ? positionStyles[orientation] : '',
        className
      )}
    >
      <div className="flex flex-col gap-1 text-center sm:text-left">
        <span className="text-[10px] font-semibold uppercase tracking-[0.4em] text-subtle">
          {badge}
        </span>
        <span className="text-lg font-semibold text-foreground">{name}</span>
        <span className="text-xs text-muted">Score {score}</span>
      </div>

      <div className="flex flex-wrap items-center justify-center gap-2 text-[11px] sm:justify-end">
        {typeof tricksWon === 'number' ? (
          <span className="rounded-full bg-black/20 px-3 py-1 font-semibold text-foreground">
            Tricks {tricksWon}
          </span>
        ) : null}
        {showBid && bid !== undefined ? (
          <span
            className={cn(
              'rounded-full px-3 py-1 font-semibold',
              bid === null
                ? 'border border-white/10 text-muted'
                : 'bg-warning/15 text-warning-contrast'
            )}
          >
            Bid {bid ?? '—'}
          </span>
        ) : null}
      </div>

      {currentCard ? (
        <div className="flex justify-center">
          <PlayingCard card={currentCard} size="sm" />
        </div>
      ) : null}

      {isViewer ? (
        <span className="self-center rounded-full bg-success/20 px-3 py-1 text-xs font-semibold uppercase tracking-wide text-success-contrast">
          You
        </span>
      ) : null}
    </div>
  )
}
