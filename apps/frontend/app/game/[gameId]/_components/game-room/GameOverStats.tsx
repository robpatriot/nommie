'use client'

import { useTranslations } from 'next-intl'
import type { Seat, Trump } from '@/lib/game-room/types'
import { cn } from '@/lib/cn'
import type { GameOverStats, PlayerStats } from '@/lib/game-room/game-stats'

const TRUMP_SYMBOLS: Record<Trump, string> = {
  CLUBS: '♣',
  DIAMONDS: '♦',
  HEARTS: '♥',
  SPADES: '♠',
  NO_TRUMPS: '—',
}

const TRUMP_COLORS: Record<Trump, string> = {
  CLUBS: 'text-slate-700 dark:text-slate-300',
  DIAMONDS: 'text-rose-600 dark:text-rose-400',
  HEARTS: 'text-rose-600 dark:text-rose-400',
  SPADES: 'text-slate-700 dark:text-slate-300',
  NO_TRUMPS: 'text-muted-foreground',
}

interface GameOverStatsProps {
  stats: GameOverStats
  playerNames: [string, string, string, string]
  seatDisplayName: (seat: Seat) => string
  winnerSeats: Seat[]
}

function PlayerCard({
  stat,
  name,
  isWinner,
  t,
}: {
  stat: PlayerStats
  name: string
  isWinner: boolean
  t: (key: string, values?: Record<string, string | number>) => string
}) {
  const bidAccuracyPct =
    stat.bidAccuracyDenominator > 0
      ? Math.round((stat.bidAccuracy / stat.bidAccuracyDenominator) * 100)
      : 0

  return (
    <div
      className={cn(
        'relative overflow-hidden rounded-2xl border bg-card/90 p-4 shadow-elevated backdrop-blur transition',
        isWinner
          ? 'border-primary/50 ring-2 ring-primary/20'
          : 'border-border/60'
      )}
    >
      {isWinner ? (
        <div className="absolute right-2 top-2 rounded-full bg-primary/20 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wider text-primary">
          {t('gameOver.stats.winner')}
        </div>
      ) : null}
      <div className="flex items-center justify-between gap-3">
        <span className="truncate font-semibold text-foreground">{name}</span>
        <span className="shrink-0 text-2xl font-bold text-primary">
          {stat.finalScore}
        </span>
      </div>
      <div className="mt-4 grid grid-cols-2 gap-3 text-sm">
        <div className="rounded-xl border border-border/40 bg-background/50 px-3 py-2">
          <p className="text-[10px] font-semibold uppercase tracking-[0.25em] text-muted-foreground">
            {t('gameOver.stats.bidAccuracy')}
          </p>
          <div className="mt-1 flex items-baseline gap-2">
            <span className="text-lg font-bold text-foreground">
              {bidAccuracyPct}%
            </span>
            <span className="text-xs text-muted-foreground">
              {stat.bidAccuracy}/{stat.bidAccuracyDenominator}
            </span>
          </div>
          <div className="mt-1.5 h-1.5 overflow-hidden rounded-full bg-muted">
            <div
              className="h-full rounded-full bg-primary/80 transition-all"
              style={{ width: `${bidAccuracyPct}%` }}
            />
          </div>
        </div>
        <div className="rounded-xl border border-border/40 bg-background/50 px-3 py-2">
          <p className="text-[10px] font-semibold uppercase tracking-[0.25em] text-muted-foreground">
            {t('gameOver.stats.roundsWon')}
          </p>
          <p className="mt-1 text-lg font-bold text-foreground">
            {stat.roundsWon}
          </p>
        </div>
        <div className="rounded-xl border border-border/40 bg-background/50 px-3 py-2">
          <p className="text-[10px] font-semibold uppercase tracking-[0.25em] text-muted-foreground">
            {t('gameOver.stats.totalTricks')}
          </p>
          <p className="mt-1 text-lg font-bold text-foreground">
            {stat.totalTricks}
          </p>
        </div>
        <div className="rounded-xl border border-border/40 bg-background/50 px-3 py-2">
          <p className="text-[10px] font-semibold uppercase tracking-[0.25em] text-muted-foreground">
            {t('gameOver.stats.biggestRound')}
          </p>
          <p className="mt-1 text-lg font-bold text-foreground">
            {stat.biggestRound}
          </p>
        </div>
      </div>
      {stat.perfectRounds > 0 ? (
        <div className="mt-3 flex items-center gap-1.5 rounded-lg bg-success/10 px-2 py-1 text-xs font-medium text-success">
          <span aria-hidden>✓</span>
          {t('gameOver.stats.perfectRounds', { count: stat.perfectRounds })}
        </div>
      ) : null}
    </div>
  )
}

export function GameOverStatsDisplay({
  stats,
  playerNames,
  seatDisplayName,
  winnerSeats,
}: GameOverStatsProps) {
  const t = useTranslations('game.gameRoom')
  const tTrump = useTranslations('game.gameRoom.trump')

  return (
    <div className="flex flex-col gap-6">
      <section className="grid gap-3 sm:grid-cols-2">
        {stats.mostCommonTrump ? (
          <div className="flex items-center gap-4 rounded-2xl border border-border/60 bg-card/80 px-4 py-3 shadow-inner">
            <span
              className={cn('text-3xl', TRUMP_COLORS[stats.mostCommonTrump])}
              aria-hidden
            >
              {TRUMP_SYMBOLS[stats.mostCommonTrump]}
            </span>
            <div>
              <p className="text-[10px] font-semibold uppercase tracking-[0.3em] text-muted-foreground">
                {t('gameOver.stats.mostCommonTrump')}
              </p>
              <p className="font-semibold text-foreground">
                {tTrump(stats.mostCommonTrump)}
              </p>
            </div>
          </div>
        ) : null}
        {stats.mostBidWins !== null ? (
          <div className="flex items-center gap-4 rounded-2xl border border-border/60 bg-card/80 px-4 py-3 shadow-inner">
            <span className="text-2xl text-amber-500" aria-hidden>
              ★
            </span>
            <div>
              <p className="text-[10px] font-semibold uppercase tracking-[0.3em] text-muted-foreground">
                {t('gameOver.stats.mostBidWins')}
              </p>
              <p className="font-semibold text-foreground">
                {seatDisplayName(stats.mostBidWins)}{' '}
                <span className="text-muted-foreground">
                  ({stats.mostBidWinsCount}×)
                </span>
              </p>
            </div>
          </div>
        ) : null}
      </section>

      <section>
        <h3 className="mb-3 text-[11px] font-semibold uppercase tracking-[0.4em] text-muted-foreground">
          {t('gameOver.stats.playerStats')}
        </h3>
        <div className="grid gap-4 sm:grid-cols-2">
          {stats.playerStats.map((stat, idx) => (
            <PlayerCard
              key={stat.seat}
              stat={stat}
              name={playerNames[idx]}
              isWinner={winnerSeats.includes(stat.seat)}
              t={t}
            />
          ))}
        </div>
      </section>
    </div>
  )
}
