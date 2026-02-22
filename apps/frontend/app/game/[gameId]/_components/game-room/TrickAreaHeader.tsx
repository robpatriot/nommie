import type { Trump } from '@/lib/game-room/types'
import { cn } from '@/lib/cn'

const suitMap = {
  CLUBS: { symbol: '♣', color: 'text-slate-900' },
  DIAMONDS: { symbol: '♦', color: 'text-rose-600' },
  HEARTS: { symbol: '♥', color: 'text-rose-600' },
  SPADES: { symbol: '♠', color: 'text-slate-900' },
} as const

interface TrickAreaHeaderProps {
  trump: Trump | null
  totalBids: number
  handSize: number
  /** When true, always show (for guide preview); otherwise hidden on lg+ viewports */
  alwaysShow?: boolean
}

export function TrickAreaHeader({
  trump,
  totalBids,
  handSize,
  alwaysShow = false,
}: TrickAreaHeaderProps) {
  const trumpDisplay = trump
    ? trump === 'NO_TRUMPS'
      ? { text: 'NT', color: 'text-foreground', isText: true }
      : {
          text: suitMap[trump].symbol,
          color: suitMap[trump].color,
          isText: false,
        }
    : { text: '—', color: 'text-foreground', isText: true }

  const isBlackSuit = trump === 'SPADES' || trump === 'CLUBS'

  const trumpElement = (
    <div
      className={cn(
        'flex h-7 shrink-0 items-center justify-center rounded-full border border-border/60 px-2',
        // base surface
        'bg-card/20',
        // subtle lightening only when needed
        isBlackSuit && 'dark:bg-foreground/18'
      )}
    >
      <span
        className={cn(
          'leading-none',
          trumpDisplay.isText ? 'text-[11px] font-semibold' : 'text-2xl',
          trumpDisplay.color
        )}
      >
        {trumpDisplay.text}
      </span>
    </div>
  )

  const bidsElement = (
    <div className="flex shrink-0">
      <div className="flex h-7 items-center justify-center rounded-full border border-border/60 bg-card/20 px-3 text-[11px] font-semibold text-foreground">
        {`${totalBids}/${handSize}`}
      </div>
    </div>
  )

  return (
    <div
      className={cn(
        'flex w-full items-center justify-between gap-2',
        !alwaysShow && 'lg:hidden'
      )}
    >
      <>
        {bidsElement}
        {trumpElement}
      </>
    </div>
  )
}
