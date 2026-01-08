'use client'

import { useEffect } from 'react'
import { useTranslations } from 'next-intl'

import type { RoundHistoryEntry, Seat } from '@/lib/game-room/types'
import { shortenNameForDisplay } from './utils'

interface ScoreHistoryDialogProps {
  isOpen: boolean
  onClose: () => void
  rounds: RoundHistoryEntry[]
  playerNames: [string, string, string, string]
  seatDisplayName: (seat: Seat) => string
  isLoading?: boolean
  error?: string | null
}

export function ScoreHistoryDialog({
  isOpen,
  onClose,
  rounds,
  playerNames,
  seatDisplayName,
  isLoading = false,
  error,
}: ScoreHistoryDialogProps) {
  const t = useTranslations('game.gameRoom.history')
  const tTrump = useTranslations('game.gameRoom.trump')

  useEffect(() => {
    if (!isOpen) {
      return
    }
    const handler = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault()
        onClose()
      }
    }
    window.addEventListener('keydown', handler)
    return () => {
      window.removeEventListener('keydown', handler)
    }
  }, [isOpen, onClose])

  if (!isOpen) {
    return null
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center px-4 py-8"
      role="dialog"
      aria-modal="true"
      aria-labelledby="score-history-title"
    >
      <div
        className="absolute inset-0 bg-overlay/60 backdrop-blur-md"
        onClick={onClose}
        aria-hidden
      />
      <div className="relative z-10 w-full max-w-5xl rounded-[36px] border border-border/70 bg-card/95 p-6 text-foreground shadow-elevated">
        <header className="flex items-start justify-between gap-4">
          <div>
            <p className="text-[11px] font-semibold uppercase tracking-[0.4em] text-muted-foreground">
              {t('kicker')}
            </p>
            <h2
              id="score-history-title"
              className="mt-2 text-3xl font-semibold text-foreground"
            >
              {t('title')}
            </h2>
            <p className="text-sm text-muted-foreground">{t('description')}</p>
          </div>
          <div className="flex items-center gap-2 rounded-full border border-border/60 bg-card px-3 py-1 text-xs text-muted-foreground">
            <span className="text-lg leading-none text-amber-400">★</span>
            <span className="uppercase tracking-[0.35em] text-[10px]">
              {t('trumpCaller')}
            </span>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded-full border border-border/60 bg-card px-3 py-1.5 text-sm font-semibold text-muted-foreground transition hover:border-primary/40 hover:text-foreground"
            aria-label={t('closeAria')}
          >
            ✕
          </button>
        </header>

        {error ? (
          <div className="mt-4 rounded-2xl border border-warning/50 bg-warning/10 px-4 py-3 text-sm text-warning-foreground">
            {error}
          </div>
        ) : null}

        <div className="mt-6 max-h-[60vh] overflow-y-auto pr-1">
          {isLoading ? (
            <div className="flex h-40 items-center justify-center text-sm text-muted-foreground">
              {t('loading')}
            </div>
          ) : rounds.length === 0 ? (
            <div className="flex h-40 items-center justify-center text-sm text-muted-foreground">
              {t('empty')}
            </div>
          ) : (
            <div className="flex flex-col gap-3 text-sm">
              <div className="grid grid-cols-[minmax(260px,1.25fr)_repeat(4,minmax(120px,0.9fr))] gap-4 rounded-2xl border border-border/60 bg-card/60 px-4 py-2 text-[11px] font-semibold uppercase tracking-[0.3em] text-muted-foreground">
                <span>{t('table.round')}</span>
                {playerNames.map((name) => (
                  <span
                    key={name}
                    className="truncate text-center"
                    title={name}
                  >
                    {shortenNameForDisplay(name, 12)}
                  </span>
                ))}
              </div>
              {rounds.map((round) => (
                <div
                  key={round.roundNo}
                  className="grid grid-cols-[minmax(260px,1.25fr)_repeat(4,minmax(120px,0.9fr))] gap-4 rounded-3xl border border-border/40 bg-background/40 p-4 shadow-sm backdrop-blur"
                >
                  <div className="flex flex-col gap-3 rounded-2xl border border-border/40 bg-card/80 p-3">
                    <div className="flex items-center justify-between">
                      <div>
                        <p className="text-[11px] font-semibold uppercase tracking-[0.35em] text-muted-foreground">
                          {t('roundCard.roundKicker', {
                            roundNo: round.roundNo,
                          })}
                        </p>
                        <p className="text-lg font-semibold text-foreground">
                          {t('roundCard.handSize', {
                            handSize: round.handSize,
                          })}
                        </p>
                      </div>
                      <div className="text-right text-xs text-muted-foreground">
                        {t('roundCard.dealer')}
                        <br />
                        <span className="font-medium text-foreground">
                          {seatDisplayName(round.dealerSeat)}
                        </span>
                      </div>
                    </div>
                    <div className="rounded-2xl border border-border/30 bg-background/60 px-3 py-2 text-xs text-muted-foreground">
                      {t('roundCard.trump')}{' '}
                      <span className="font-semibold text-foreground">
                        {round.trump
                          ? tTrump(round.trump)
                          : tTrump('undeclared')}
                      </span>{' '}
                      {round.trumpSelectorSeat !== null ? (
                        <>
                          {t('roundCard.by')}{' '}
                          <span className="font-medium text-foreground">
                            {seatDisplayName(round.trumpSelectorSeat)}
                          </span>
                        </>
                      ) : (
                        t('roundCard.pending')
                      )}
                    </div>
                  </div>
                  {round.bids.map((bid, index) => {
                    const seatIndex = index as Seat
                    const isSelector = round.trumpSelectorSeat === seatIndex
                    const cumulative = round.cumulativeScores[seatIndex]
                    return (
                      <div
                        key={`${round.roundNo}-${playerNames[seatIndex]}`}
                        className="rounded-2xl border border-border/40 bg-card/70 px-3 py-2 text-center"
                      >
                        <div className="text-sm font-semibold text-foreground">
                          <span className="flex items-center justify-center gap-1">
                            <span>
                              {t('roundCard.bid', { bid: bid ?? '—' })}
                            </span>
                            {isSelector ? (
                              <span
                                className="text-base leading-none text-amber-400"
                                aria-label={t('trumpCaller')}
                                title={t('trumpCaller')}
                              >
                                ★
                              </span>
                            ) : null}
                          </span>
                        </div>
                        <div className="text-[10px] uppercase tracking-[0.3em] text-muted-foreground">
                          {t('roundCard.total')}
                        </div>
                        <div className="text-2xl font-bold text-foreground">
                          {cumulative}
                        </div>
                      </div>
                    )
                  })}
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
