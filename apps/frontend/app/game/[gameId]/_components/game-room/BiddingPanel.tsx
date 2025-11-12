'use client'

import { type FormEvent, useEffect, useMemo, useState } from 'react'
import type { BiddingSnapshot, Seat } from '@/lib/game-room/types'
import { getOrientation } from './utils'
import type { GameRoomViewProps } from '../game-room-view'

interface BiddingPanelProps {
  phase: BiddingSnapshot
  viewerSeat: Seat
  layoutSeat: Seat
  playerNames: [string, string, string, string]
  bidding: NonNullable<GameRoomViewProps['biddingState']>
}

export function BiddingPanel({
  phase,
  viewerSeat,
  layoutSeat,
  playerNames,
  bidding,
}: BiddingPanelProps) {
  const minBid = phase.min_bid
  const maxBid = phase.max_bid
  const viewerBid = phase.bids[viewerSeat] ?? null
  const isViewerTurn = phase.to_act === viewerSeat
  const activeName =
    phase.to_act === viewerSeat ? 'You' : playerNames[phase.to_act]
  const [selectedBid, setSelectedBid] = useState<number>(
    () => viewerBid ?? minBid
  )

  useEffect(() => {
    if (viewerBid !== null) {
      setSelectedBid(viewerBid)
      return
    }

    setSelectedBid((current) => {
      if (current < minBid) return minBid
      if (current > maxBid) return maxBid
      return current
    })
  }, [maxBid, minBid, viewerBid])

  const seatBids = useMemo(
    () =>
      ([0, 1, 2, 3] as const).map((seat) => ({
        seat,
        name: seat === viewerSeat ? 'You' : playerNames[seat],
        bid: phase.bids[seat],
        orientation: getOrientation(layoutSeat, seat),
      })),
    [layoutSeat, phase.bids, playerNames, viewerSeat]
  )

  const isSubmitDisabled =
    !isViewerTurn || viewerBid !== null || bidding.isPending

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()

    if (isSubmitDisabled) {
      return
    }

    const normalizedBid = Math.min(Math.max(selectedBid, minBid), maxBid)
    setSelectedBid(normalizedBid)
    await bidding.onSubmit(normalizedBid)
  }

  return (
    <section className="mx-auto flex w-full max-w-4xl flex-col gap-4 rounded-2xl border border-success/40 bg-success/10 p-4">
      <header className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <h2 className="text-sm font-semibold uppercase tracking-wide text-success-foreground">
            Bidding
          </h2>
          <p className="text-xs text-success-foreground/80">
            Select your bid between {minBid} and {maxBid}. Once submitted, the
            next player will be prompted automatically.
          </p>
        </div>
        <div className="rounded-full border border-success/40 bg-success/15 px-3 py-1 text-xs font-medium text-success-foreground">
          Waiting on: {activeName}
        </div>
      </header>

      <form
        className="flex flex-col gap-3 rounded-lg border border-success/30 bg-surface/60 p-4 shadow-inner shadow-success/20"
        onSubmit={handleSubmit}
      >
        <label
          htmlFor="bid-value"
          className="text-xs font-medium uppercase tracking-wide text-success-foreground"
        >
          Your Bid
        </label>
        <div className="flex flex-wrap items-center gap-3">
          <input
            id="bid-value"
            type="number"
            min={minBid}
            max={maxBid}
            step={1}
            value={selectedBid}
            onChange={(event) => setSelectedBid(Number(event.target.value))}
            className="w-24 rounded-md border border-success/40 bg-background px-3 py-2 text-sm font-semibold text-foreground outline-none transition focus:border-success focus:ring focus:ring-success/40 disabled:cursor-not-allowed disabled:opacity-60"
            disabled={viewerBid !== null || bidding.isPending || !isViewerTurn}
            aria-label="Bid value"
            aria-describedby="bid-range-hint"
          />
          <button
            type="submit"
            className="rounded-md bg-success px-4 py-2 text-sm font-semibold text-success-foreground transition hover:bg-success/80 disabled:cursor-not-allowed disabled:bg-success/40 disabled:text-success-foreground/70"
            disabled={isSubmitDisabled}
            aria-label={
              bidding.isPending
                ? 'Submitting bid'
                : `Submit bid of ${selectedBid}`
            }
          >
            {bidding.isPending ? 'Submitting…' : 'Submit Bid'}
          </button>
        </div>
        <p id="bid-range-hint" className="text-xs text-success-foreground/80">
          Allowed range: {minBid} – {maxBid}.{' '}
          {isViewerTurn
            ? viewerBid === null
              ? 'Choose a value and submit before time runs out.'
              : 'Bid submitted — waiting for other players.'
            : `Waiting for ${activeName} to bid.`}
        </p>
      </form>

      <div className="rounded-lg border border-success/20 bg-surface/60 p-4">
        <h3 className="mb-3 text-xs font-semibold uppercase tracking-wide text-success-foreground">
          Bid Tracker
        </h3>
        <ul className="grid gap-2 sm:grid-cols-2">
          {seatBids.map(({ seat, name, bid, orientation }) => (
            <li
              key={seat}
              className={`flex items-center justify-between rounded-md border border-border bg-surface/60 px-3 py-2 text-sm ${
                seat === phase.to_act
                  ? 'border-success bg-success/10 text-success-foreground'
                  : 'text-muted'
              }`}
            >
              <div className="flex flex-col">
                <span className="font-medium text-foreground">{name}</span>
                <span className="text-[10px] uppercase text-subtle">
                  {orientation}
                </span>
              </div>
              <span className="text-sm font-semibold text-foreground">
                {bid ?? '—'}
              </span>
            </li>
          ))}
        </ul>
      </div>
    </section>
  )
}
