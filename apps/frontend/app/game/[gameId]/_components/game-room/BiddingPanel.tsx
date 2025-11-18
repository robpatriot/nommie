'use client'

import {
  type ChangeEvent,
  type FormEvent,
  useEffect,
  useMemo,
  useState,
} from 'react'
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
  const handSize = phase.round.hand_size
  const viewerBid = phase.bids[viewerSeat] ?? null
  const zeroBidLocked = bidding.zeroBidLocked ?? false
  const isViewerTurn = phase.to_act === viewerSeat
  const activeName =
    phase.to_act === viewerSeat ? 'You' : playerNames[phase.to_act]
  const [bidInput, setBidInput] = useState<string>(() =>
    (viewerBid ?? minBid).toString()
  )
  const [flashValidation, setFlashValidation] = useState(false)

  useEffect(() => {
    if (viewerBid !== null) {
      setBidInput(viewerBid.toString())
      return
    }

    setBidInput((current) => {
      if (current.trim() === '') {
        return current
      }

      const parsed = Number(current)
      if (!Number.isFinite(parsed)) {
        return ''
      }

      if (parsed < minBid) return String(minBid)
      if (parsed > maxBid) return String(maxBid)
      return current
    })
  }, [maxBid, minBid, viewerBid])

  useEffect(() => {
    if (!flashValidation) {
      return
    }

    const timeout = window.setTimeout(() => setFlashValidation(false), 600)
    return () => window.clearTimeout(timeout)
  }, [flashValidation])

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

  const remainingNullBids = phase.bids.filter((bid) => bid === null).length
  const isFinalBid = remainingNullBids === 1
  const sumOfOtherBids = phase.bids.reduce<number>((total, bid, seatIndex) => {
    if (seatIndex === viewerSeat) {
      return total
    }
    return total + (bid ?? 0)
  }, 0)

  const parsedBid =
    bidInput.trim() === '' ? null : Number.parseInt(bidInput, 10)
  const hitsHandSize =
    isFinalBid && parsedBid !== null && sumOfOtherBids + parsedBid === handSize

  const validationMessages: string[] = []

  if (parsedBid === null) {
    validationMessages.push('Enter a bid before submitting.')
  } else {
    if (parsedBid < minBid) {
      validationMessages.push(`Bid must be at least ${minBid}.`)
    }
    if (parsedBid > maxBid) {
      validationMessages.push(`Bid cannot exceed ${maxBid}.`)
    }
    if (parsedBid === 0 && zeroBidLocked) {
      validationMessages.push("You've bid 0 the maximum number of times.")
    }
    if (hitsHandSize) {
      validationMessages.push(
        `Total bids cannot equal ${handSize}. Choose another number.`
      )
    }
  }

  const warningMessage = validationMessages[0] ?? null
  const hasValidationIssue = warningMessage !== null
  const describedByIds = warningMessage
    ? 'bid-range-hint bid-validation-warning'
    : 'bid-range-hint'

  const isSubmitDisabled =
    !isViewerTurn || viewerBid !== null || bidding.isPending

  const handleInputChange = (event: ChangeEvent<HTMLInputElement>) => {
    const value = event.target.value
    if (value === '' || /^\d+$/.test(value)) {
      setBidInput(value)
    }
  }

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()

    if (isSubmitDisabled) {
      return
    }

    if (parsedBid === null || hasValidationIssue) {
      setFlashValidation(true)
      return
    }

    const normalizedBid = Math.min(Math.max(parsedBid, minBid), maxBid)
    setBidInput(normalizedBid.toString())
    await bidding.onSubmit(normalizedBid)
  }

  return (
    <section className="mx-auto flex w-full max-w-4xl flex-col gap-4 rounded-3xl border border-success/50 bg-success/15 p-5 shadow-[0_30px_90px_rgba(56,189,116,0.25)]">
      <header className="flex flex-wrap items-center justify-between gap-2 text-success-contrast">
        <div>
          <h2 className="text-sm font-semibold uppercase tracking-[0.4em] text-success-contrast">
            Bidding
          </h2>
          <p className="text-xs text-success-contrast/80">
            Select your bid between {minBid} and {maxBid}. Once submitted, the
            next player will be prompted automatically.
          </p>
        </div>
        <div className="rounded-full border border-success/60 bg-success/25 px-3 py-1 text-xs font-semibold text-success-contrast">
          Waiting on: {activeName}
        </div>
      </header>

      <form
        className="flex flex-col gap-3 rounded-2xl border border-success/30 bg-surface/85 p-4 shadow-inner shadow-success/20"
        onSubmit={handleSubmit}
      >
        <label
          htmlFor="bid-value"
          className="text-xs font-medium uppercase tracking-wide text-success-contrast"
        >
          Your bid
        </label>
        <div className="flex flex-wrap items-center gap-3">
          <input
            id="bid-value"
            type="text"
            inputMode="numeric"
            pattern="[0-9]*"
            value={bidInput}
            onChange={handleInputChange}
            className={`w-24 rounded-xl border bg-background px-3 py-2 text-sm font-semibold text-foreground outline-none transition disabled:cursor-not-allowed disabled:opacity-60 ${
              hasValidationIssue
                ? 'border-warning/70 focus:border-warning focus:ring focus:ring-warning/30'
                : 'border-success/40 focus:border-success focus:ring focus:ring-success/40'
            } ${flashValidation && hasValidationIssue ? 'animate-pulse' : ''}`}
            disabled={viewerBid !== null || bidding.isPending || !isViewerTurn}
            aria-label="Bid value"
            aria-describedby={describedByIds}
            aria-invalid={hasValidationIssue}
          />
          <button
            type="submit"
            className="rounded-2xl bg-success px-4 py-2 text-sm font-semibold text-success-foreground shadow-lg shadow-success/40 transition hover:bg-success/80 disabled:cursor-not-allowed disabled:bg-success/40 disabled:text-success-foreground/70"
            disabled={isSubmitDisabled}
            aria-label={
              bidding.isPending
                ? 'Submitting bid'
                : parsedBid !== null
                  ? `Submit bid of ${parsedBid}`
                  : 'Submit bid'
            }
          >
            {bidding.isPending ? 'Submitting…' : 'Submit bid'}
          </button>
        </div>
        <p id="bid-range-hint" className="text-xs text-success-contrast/80">
          Allowed range: {minBid} – {maxBid}.{' '}
          {isViewerTurn
            ? viewerBid === null
              ? "Choose a value and submit when you're ready."
              : 'Bid submitted — waiting for other players.'
            : `Waiting for ${activeName} to bid.`}
        </p>
        {warningMessage ? (
          <p
            id="bid-validation-warning"
            className={`text-xs font-semibold text-warning-contrast ${
              flashValidation ? 'animate-pulse' : ''
            }`}
            role="alert"
          >
            {warningMessage}
          </p>
        ) : null}
      </form>

      <div className="rounded-2xl border border-success/20 bg-surface/70 p-4">
        <h3 className="mb-3 text-xs font-semibold uppercase tracking-wide text-success-contrast">
          Bid tracker
        </h3>
        <ul className="grid gap-2 sm:grid-cols-2">
          {seatBids.map(({ seat, name, bid, orientation }) => (
            <li
              key={seat}
              className={`flex items-center justify-between rounded-xl border border-border bg-surface/80 px-3 py-2 text-sm ${
                seat === phase.to_act
                  ? 'border-success bg-success/10 text-success-contrast'
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
