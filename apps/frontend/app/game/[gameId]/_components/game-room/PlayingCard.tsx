import { useTranslations } from 'next-intl'
import type { Card } from '@/lib/game-room/types'
import { cn } from '@/lib/cn'

const suitMap = {
  S: { symbol: '♠', color: 'text-slate-900' },
  H: { symbol: '♥', color: 'text-rose-600' },
  D: { symbol: '♦', color: 'text-rose-600' },
  C: { symbol: '♣', color: 'text-slate-900' },
} as const

export const CARD_DIMENSIONS = {
  sm: { width: 56, height: 80 },
  md: { width: 80, height: 112 },
  lg: { width: 96, height: 144 },
} as const

const sizeStyles = {
  sm: {
    card: 'h-20 w-14 text-2xl',
    corner: 'text-[10px]',
    symbol: 'text-2xl',
  },
  md: {
    card: 'h-28 w-20 text-3xl',
    corner: 'text-xs',
    symbol: 'text-4xl',
  },
  lg: {
    card: 'h-36 w-24 text-4xl',
    corner: 'text-sm',
    symbol: 'text-5xl',
  },
} as const

type PlayingCardProps = {
  card: Card
  size?: keyof typeof sizeStyles
  className?: string
  isDimmed?: boolean
  isSelected?: boolean
}

function getRankLabel(card: Card) {
  const value = card.slice(0, -1)
  if (value === 'T') return '10'
  return value || card
}

export function PlayingCard({
  card,
  size = 'md',
  className,
  isDimmed,
  isSelected,
}: PlayingCardProps) {
  const t = useTranslations('game.gameRoom')
  const tCards = useTranslations('game.cards')
  const suitKey = card.slice(-1) as keyof typeof suitMap
  const suit = suitMap[suitKey] ?? suitMap.S
  const rankLabel = getRankLabel(card)
  const styles = sizeStyles[size]

  // Map suit keys to translation keys
  const suitLabelKey =
    suitKey === 'S'
      ? 'SPADES'
      : suitKey === 'H'
        ? 'HEARTS'
        : suitKey === 'D'
          ? 'DIAMONDS'
          : 'CLUBS'
  const suitLabel = t(`trump.${suitLabelKey}`)

  return (
    <div
      className={cn(
        'relative isolate flex flex-col items-center justify-center rounded-[1.35rem] border-2 border-white/70 bg-gradient-to-b from-[#fffcf8] via-[#fdfcfd] to-[#eef2f7] text-slate-900 shadow-[0_18px_35px_rgba(0,0,0,0.45)] transition-all',
        'before:absolute before:inset-2 before:rounded-[1rem] before:border before:border-white/30 before:bg-white/40 before:content-[""]',
        styles.card,
        suitKey === 'S' || suitKey === 'C'
          ? 'text-slate-900 drop-shadow-[0_6px_12px_rgba(0,0,0,0.25)]'
          : 'drop-shadow-[0_6px_12px_rgba(184,28,28,0.35)]',
        isDimmed
          ? '[filter:grayscale(60%)_brightness(0.75)] saturate-[75%]'
          : '',
        isSelected ? 'scale-[1.02]' : '',
        className
      )}
      aria-label={tCards('aria.rankOfSuit', {
        rank: rankLabel,
        suit: suitLabel,
      })}
    >
      <span
        className={cn(
          'pointer-events-none absolute left-2 top-2 flex flex-col font-semibold leading-tight text-slate-800',
          styles.corner
        )}
      >
        <span>{rankLabel}</span>
        <span className={cn('text-base', suit.color)}>{suit.symbol}</span>
      </span>
      <span
        className={cn(
          'pointer-events-none font-semibold',
          suit.color,
          styles.symbol
        )}
      >
        {suit.symbol}
      </span>
      <span
        className={cn(
          'pointer-events-none absolute bottom-2 right-2 rotate-180 flex flex-col font-semibold leading-tight text-slate-800',
          styles.corner
        )}
      >
        <span>{rankLabel}</span>
        <span className={cn('text-base', suit.color)}>{suit.symbol}</span>
      </span>
    </div>
  )
}
