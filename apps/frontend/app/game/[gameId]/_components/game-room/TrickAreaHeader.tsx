import type { Trump } from '@/lib/game-room/types'
import { cn } from '@/lib/cn'
import { useMediaQuery } from '@/hooks/useMediaQuery'

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
}

export function TrickAreaHeader({
  trump,
  totalBids,
  handSize,
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

  // For 640px-1024px: trump left, bids right. For <640px: bids left, trump right
  const isMediumViewport = useMediaQuery(
    '(min-width: 640px) and (max-width: 1023px)'
  )

  const trumpElement = (
    <div className="flex h-7 shrink-0 items-center justify-center rounded-full border border-white/20 bg-white/20 px-2">
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
      <div className="flex h-7 items-center justify-center rounded-full border border-white/20 bg-white/20 px-3 text-[11px] font-semibold text-foreground">
        {`${totalBids}/${handSize}`}
      </div>
    </div>
  )

  return (
    <div className="flex w-full items-center justify-between gap-2 lg:hidden">
      {isMediumViewport ? (
        <>
          {trumpElement}
          {bidsElement}
        </>
      ) : (
        <>
          {bidsElement}
          {trumpElement}
        </>
      )}
    </div>
  )
}
