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

  return (
    <section className="mx-auto flex w-full max-w-4xl flex-col gap-4 rounded-2xl border border-accent/40 bg-accent/10 p-4">
      <header className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <h2 className="text-sm font-semibold uppercase tracking-wide text-accent-foreground">
            Select Trump
          </h2>
          <p className="text-xs text-accent-foreground/80">
            Choose the trump suit for this round. Trump cards outrank all other
            suits.
          </p>
        </div>
        <div className="rounded-full border border-accent/40 bg-accent/15 px-3 py-1 text-xs font-medium text-accent-foreground">
          Waiting on: {activeName}
        </div>
      </header>

      <form
        className="flex flex-col gap-3 rounded-lg border border-accent/30 bg-surface/60 p-4 shadow-inner shadow-accent/20"
        onSubmit={handleSubmit}
      >
        <div className="flex flex-wrap gap-2">
          {allowedTrumps.map((option) => {
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
                  setSelectedTrump(option)
                }}
                disabled={disabled}
                className={`rounded-md border px-3 py-2 text-sm font-semibold transition ${
                  isSelected
                    ? 'border-accent bg-accent/30 text-accent-foreground shadow-md shadow-accent/30'
                    : canSelect
                      ? 'border-accent/40 bg-surface text-accent-foreground hover:border-accent hover:bg-accent/15'
                      : 'border-border bg-surface text-muted'
                } ${
                  disabled ? 'cursor-not-allowed opacity-60' : 'cursor-pointer'
                }`}
                aria-pressed={isSelected}
              >
                {formatTrump(option)}
              </button>
            )
          })}
        </div>

        <button
          type="submit"
          className="w-full rounded-md bg-accent px-4 py-2 text-sm font-semibold text-accent-foreground transition hover:bg-accent/80 disabled:cursor-not-allowed disabled:bg-accent/40 disabled:text-accent-foreground/70"
          disabled={!canSelect || isPending || !selectedTrump}
        >
          {submitLabel}
        </button>

        <p className="text-xs text-accent-foreground/75">
          {canSelect
            ? 'Select a trump suit and confirm to continue to trick play.'
            : `Waiting for ${activeName} to choose the trump suit.`}
        </p>
      </form>
    </section>
  )
}
