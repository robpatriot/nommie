import { cn } from '@/lib/cn'
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
  const { orientation, name, isViewer, tricksWon, isActive } = summary

  const positionStyles: Record<SeatSummary['orientation'], string> = {
    top: 'lg:col-start-2 lg:row-start-1 lg:justify-self-center lg:self-center',
    left: 'lg:col-start-1 lg:row-start-2 lg:justify-self-center lg:self-center',
    right:
      'lg:col-start-3 lg:row-start-2 lg:justify-self-center lg:self-center',
    bottom:
      'lg:col-start-2 lg:row-start-3 lg:justify-self-center lg:self-center',
  }

  const badge =
    orientation === 'bottom'
      ? isViewer
        ? 'You'
        : 'South seat'
      : `${orientation.charAt(0).toUpperCase()}${orientation.slice(1)} seat`

  const baseClasses =
    variant === 'table'
      ? 'w-fit items-center text-center'
      : 'max-w-none text-left sm:flex-row sm:items-center sm:justify-between'

  return (
    <div
      className={cn(
        'flex w-fit flex-col gap-1.5 rounded-xl border border-white/10 bg-surface/80 px-3 py-2 text-sm text-muted shadow-[0_18px_65px_rgba(0,0,0,0.35)] backdrop-blur transition',
        baseClasses,
        isActive
          ? 'ring-2 ring-success/80 ring-offset-1 ring-offset-surface'
          : '',
        variant === 'table' ? positionStyles[orientation] : '',
        className
      )}
    >
      <div className="flex flex-col gap-0.5 text-center sm:text-left">
        <span className="text-[10px] font-semibold uppercase tracking-[0.4em] text-subtle">
          {badge}
        </span>
        <span className="text-sm font-semibold text-foreground">{name}</span>
      </div>

      <div className="flex flex-wrap items-center justify-center gap-1 text-[10px] sm:justify-end">
        {typeof tricksWon === 'number' ? (
          <span className="rounded-full bg-black/20 px-2 py-0.5 font-semibold text-foreground">
            Tricks {tricksWon}
          </span>
        ) : null}
      </div>

      {isViewer ? (
        <span className="self-center rounded-full bg-success/20 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-success-contrast">
          You
        </span>
      ) : null}
    </div>
  )
}
