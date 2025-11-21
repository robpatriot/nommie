'use client'

import { useEffect } from 'react'

import type { RoundHistoryEntry, Seat } from '@/lib/game-room/types'
import { formatTrump, shortenNameForDisplay } from './utils'

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
        className="absolute inset-0 bg-black/60 backdrop-blur-md"
        onClick={onClose}
        aria-hidden
      />
      <div className="relative z-10 w-full max-w-5xl rounded-[36px] border border-white/15 bg-surface/95 p-6 text-foreground shadow-[0_40px_160px_rgba(0,0,0,0.55)]">
        <header className="flex items-start justify-between gap-4">
          <div>
            <p className="text-[11px] font-semibold uppercase tracking-[0.4em] text-subtle">
              Score archive
            </p>
            <h2
              id="score-history-title"
              className="mt-2 text-3xl font-semibold text-foreground"
            >
              Score sheet
            </h2>
            <p className="text-sm text-muted">
              Review bids, trump picks, and the running totals for every round.
            </p>
          </div>
          <div className="flex items-center gap-2 rounded-full border border-border/60 bg-surface px-3 py-1 text-xs text-muted">
            <span className="text-lg leading-none text-amber-400">★</span>
            <span className="uppercase tracking-[0.35em] text-[10px]">
              Trump caller
            </span>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded-full border border-border/60 bg-surface px-3 py-1.5 text-sm font-semibold text-muted transition hover:border-primary/40 hover:text-foreground"
            aria-label="Close score history"
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
            <div className="flex h-40 items-center justify-center text-sm text-muted">
              Loading history…
            </div>
          ) : rounds.length === 0 ? (
            <div className="flex h-40 items-center justify-center text-sm text-muted">
              No completed rounds yet. Start playing to build the score sheet.
            </div>
          ) : (
            <div className="flex flex-col gap-3 text-sm">
              <div className="grid grid-cols-[minmax(260px,1.25fr)_repeat(4,minmax(120px,0.9fr))] gap-4 rounded-2xl border border-border/60 bg-surface/60 px-4 py-2 text-[11px] font-semibold uppercase tracking-[0.3em] text-subtle">
                <span>Round</span>
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
                  <div className="flex flex-col gap-3 rounded-2xl border border-border/40 bg-surface/80 p-3">
                    <div className="flex items-center justify-between">
                      <div>
                        <p className="text-[11px] font-semibold uppercase tracking-[0.35em] text-subtle">
                          Round {round.roundNo}
                        </p>
                        <p className="text-lg font-semibold text-foreground">
                          {round.handSize} cards
                        </p>
                      </div>
                      <div className="text-right text-xs text-muted">
                        Dealer
                        <br />
                        <span className="font-medium text-foreground">
                          {seatDisplayName(round.dealerSeat)}
                        </span>
                      </div>
                    </div>
                    <div className="rounded-2xl border border-border/30 bg-background/60 px-3 py-2 text-xs text-muted">
                      Trump{' '}
                      <span className="font-semibold text-foreground">
                        {formatTrump(round.trump)}
                      </span>{' '}
                      {round.trumpSelectorSeat !== null ? (
                        <>
                          by{' '}
                          <span className="font-medium text-foreground">
                            {seatDisplayName(round.trumpSelectorSeat)}
                          </span>
                        </>
                      ) : (
                        '(pending)'
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
                        className="rounded-2xl border border-border/40 bg-surface/70 px-3 py-2 text-center"
                      >
                        <div className="text-sm font-semibold text-foreground">
                          <span className="flex items-center justify-center gap-1">
                            <span>Bid {bid ?? '—'}</span>
                            {isSelector ? (
                              <span
                                className="text-base leading-none text-amber-400"
                                aria-label="Trump caller"
                                title="Trump caller"
                              >
                                ★
                              </span>
                            ) : null}
                          </span>
                        </div>
                        <div className="text-[10px] uppercase tracking-[0.3em] text-subtle">
                          Total
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
