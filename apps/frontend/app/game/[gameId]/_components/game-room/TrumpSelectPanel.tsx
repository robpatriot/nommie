'use client'

import { type FormEvent, useEffect, useState } from 'react'
import type { Seat, Trump, TrumpSelectSnapshot } from '@/lib/game-room/types'
import { formatTrump } from './utils'
import type { GameRoomViewProps } from '../game-room-view'

interface TrumpSelectPanelProps {
  phase: TrumpSelectSnapshot
  viewerSeat: Seat
  playerNames: [string, string, string, string]
  trump?: GameRoomViewProps['trumpState']
}

export function TrumpSelectPanel({
  phase,
  viewerSeat,
  playerNames,
  trump,
}: TrumpSelectPanelProps) {
  const allowedTrumps = phase.allowed_trumps
  const [selectedTrump, setSelectedTrump] = useState<Trump | null>(
    () => allowedTrumps[0] ?? null
  )

  useEffect(() => {
    if (allowedTrumps.length === 0) {
      setSelectedTrump(null)
      return
    }
    setSelectedTrump((current) => {
      if (current && allowedTrumps.includes(current)) {
        return current
      }
      return allowedTrumps[0] ?? null
    })
  }, [allowedTrumps])

  const isViewerTurn = phase.to_act === viewerSeat
  const activeName = isViewerTurn ? 'You' : playerNames[phase.to_act]
  const canSelect = trump?.canSelect ?? false
  const isPending = trump?.isPending ?? false

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (!selectedTrump || !canSelect || !trump?.onSelect) {
      return
    }

    await trump.onSelect(selectedTrump)
  }

  const submitLabel = isPending
    ? 'Choosing…'
    : canSelect
      ? 'Confirm Trump'
      : `Waiting for ${activeName}`

  // Helper to get suit symbol
  const getSuitSymbol = (trump: Trump): string => {
    switch (trump) {
      case 'CLUBS':
        return '♣'
      case 'DIAMONDS':
        return '♦'
      case 'HEARTS':
        return '♥'
      case 'SPADES':
        return '♠'
      case 'NO_TRUMP':
        return 'NT'
      default:
        return ''
    }
  }

  // Helper to get suit color classes
  const getSuitColor = (trump: Trump): string => {
    switch (trump) {
      case 'HEARTS':
      case 'DIAMONDS':
        return 'text-rose-600'
      case 'CLUBS':
      case 'SPADES':
        return 'text-slate-900'
      case 'NO_TRUMP':
        return 'text-accent-contrast'
      default:
        return ''
    }
  }

  // Separate suits from no trump
  const suits = allowedTrumps.filter((t) => t !== 'NO_TRUMP').reverse()
  const hasNoTrump = allowedTrumps.includes('NO_TRUMP')

  return (
    <section className="flex w-full flex-col gap-4 rounded-3xl border border-accent/50 bg-accent/15 p-5 text-accent-contrast shadow-[0_30px_90px_rgba(94,234,212,0.25)]">
      <header>
        <h2 className="text-sm font-semibold uppercase tracking-[0.4em]">
          Select trump
        </h2>
        <p className="text-xs text-accent-contrast/80">
          Choose the trump suit for this round. Trump cards outrank all other
          suits.
        </p>
      </header>

      <form
        className="flex flex-col gap-3 rounded-2xl border border-accent/30 bg-surface/85 p-4 shadow-inner shadow-accent/20"
        onSubmit={handleSubmit}
      >
        <div className="flex flex-col gap-3">
          {/* Suits row */}
          {suits.length > 0 && (
            <div
              className="grid gap-2"
              style={{
                gridTemplateColumns: `repeat(${suits.length}, minmax(0, 1fr))`,
              }}
            >
              {suits.map((option) => {
                const isSelected = option === selectedTrump
                const disabled = !canSelect || isPending
                return (
                  <button
                    key={option}
                    type="button"
                    onClick={() => {
                      if (disabled) {
                        return
                      }
                      setSelectedTrump(isSelected ? null : option)
                    }}
                    disabled={disabled}
                    className={`flex items-center justify-center rounded-2xl border px-4 py-3 text-center transition ${
                      isSelected
                        ? 'border-accent bg-accent/30 text-accent-contrast shadow-md shadow-accent/30'
                        : canSelect
                          ? 'border-accent/40 bg-surface text-accent-contrast hover:border-accent hover:bg-accent/15'
                          : 'border-border bg-surface text-muted'
                    } ${
                      disabled
                        ? 'cursor-not-allowed opacity-60'
                        : 'cursor-pointer'
                    }`}
                    aria-label={`Select ${formatTrump(option)} as trump suit${isSelected ? ', currently selected' : ''}`}
                    aria-pressed={isSelected}
                  >
                    <span
                      className={`text-5xl font-semibold ${getSuitColor(option)}`}
                    >
                      {getSuitSymbol(option)}
                    </span>
                  </button>
                )
              })}
            </div>
          )}

          {/* No Trump row */}
          {hasNoTrump && (
            <div className="grid gap-2" style={{ gridTemplateColumns: '1fr' }}>
              <button
                type="button"
                onClick={() => {
                  if (!canSelect || isPending) {
                    return
                  }
                  setSelectedTrump(
                    selectedTrump === 'NO_TRUMP' ? null : 'NO_TRUMP'
                  )
                }}
                disabled={!canSelect || isPending}
                className={`flex items-center justify-center rounded-2xl border px-4 py-3 text-center transition ${
                  selectedTrump === 'NO_TRUMP'
                    ? 'border-accent bg-accent/30 text-accent-contrast shadow-md shadow-accent/30'
                    : canSelect
                      ? 'border-accent/40 bg-surface text-accent-contrast hover:border-accent hover:bg-accent/15'
                      : 'border-border bg-surface text-muted'
                } ${
                  !canSelect || isPending
                    ? 'cursor-not-allowed opacity-60'
                    : 'cursor-pointer'
                }`}
                aria-label={`Select ${formatTrump('NO_TRUMP')} as trump${selectedTrump === 'NO_TRUMP' ? ', currently selected' : ''}`}
                aria-pressed={selectedTrump === 'NO_TRUMP'}
              >
                <span className="text-xl font-semibold text-accent-contrast">
                  No Trumps
                </span>
              </button>
            </div>
          )}
        </div>

        <button
          type="submit"
          className="w-full rounded-2xl bg-accent px-4 py-3 text-sm font-semibold text-accent-foreground shadow-lg shadow-accent/40 transition hover:bg-accent/80 disabled:cursor-not-allowed disabled:bg-accent/40 disabled:text-accent-foreground/70"
          disabled={!canSelect || isPending || !selectedTrump}
          aria-label={
            isPending
              ? 'Selecting trump suit'
              : selectedTrump
                ? `Confirm ${formatTrump(selectedTrump)} as trump suit`
                : 'Select trump suit'
          }
        >
          {submitLabel}
        </button>

        <p className="text-xs text-accent-contrast/75">
          {canSelect
            ? 'Select a trump suit and confirm to continue to trick play.'
            : `Waiting for ${activeName} to choose the trump suit.`}
        </p>
      </form>
    </section>
  )
}
