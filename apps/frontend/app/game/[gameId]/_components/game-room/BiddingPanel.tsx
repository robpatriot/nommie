'use client'

import {
  type ChangeEvent,
  type FormEvent,
  startTransition,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react'
import { useTranslations } from 'next-intl'
import type { BiddingSnapshot, Seat } from '@/lib/game-room/types'
import { getPlayerDisplayName } from '@/utils/player-names'
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
  const t = useTranslations('game.gameRoom.bidding')
  const tYou = useTranslations('game.gameRoom')
  const minBid = phase.min_bid
  const maxBid = phase.max_bid
  const handSize = phase.round.hand_size
  const viewerBid = phase.bids[viewerSeat] ?? null
  const zeroBidLocked = bidding.zeroBidLocked ?? false
  const isViewerTurn = phase.to_act === viewerSeat
  const activeName = getPlayerDisplayName(
    phase.to_act,
    viewerSeat,
    playerNames,
    tYou('you')
  )
  const [bidInput, setBidInput] = useState<string>('')
  const [flashValidation, setFlashValidation] = useState(false)
  const [hasTyped, setHasTyped] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)

  // Reset input when bid is submitted
  // Use startTransition to mark as non-urgent update to avoid cascading renders
  useEffect(() => {
    if (viewerBid !== null) {
      startTransition(() => {
        setBidInput('')
        setHasTyped(false)
      })
    }
  }, [viewerBid])

  useEffect(() => {
    if (isViewerTurn && viewerBid === null) {
      inputRef.current?.focus()
    }
  }, [isViewerTurn, viewerBid])

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
        name: getPlayerDisplayName(seat, viewerSeat, playerNames, tYou('you')),
        bid: phase.bids[seat],
        orientation: getOrientation(layoutSeat, seat),
      })),
    [layoutSeat, phase.bids, playerNames, viewerSeat, tYou]
  )

  const { remainingNullBids, sumOfOtherBids } = useMemo(() => {
    const remainingNullBids = phase.bids.filter((bid) => bid === null).length
    const sumOfOtherBids = phase.bids.reduce<number>(
      (total, bid, seatIndex) => {
        if (seatIndex === viewerSeat) {
          return total
        }
        return total + (bid ?? 0)
      },
      0
    )
    return { remainingNullBids, sumOfOtherBids }
  }, [phase.bids, viewerSeat])
  const isFinalBid = remainingNullBids === 1

  const parsedBid =
    bidInput.trim() === '' ? null : Number.parseInt(bidInput, 10)
  const hitsHandSize =
    isFinalBid && parsedBid !== null && sumOfOtherBids + parsedBid === handSize

  const validationMessages: string[] = []

  if (parsedBid === null) {
    if (hasTyped) {
      validationMessages.push(t('validation.enterBid'))
    }
  } else {
    if (parsedBid < minBid) {
      validationMessages.push(t('validation.minBid', { minBid }))
    }
    if (parsedBid > maxBid) {
      validationMessages.push(t('validation.maxBid', { maxBid }))
    }
    if (parsedBid === 0 && zeroBidLocked) {
      validationMessages.push(t('validation.zeroBidLocked'))
    }
    if (hitsHandSize) {
      validationMessages.push(
        t('validation.totalCannotEqualHandSize', { handSize })
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
      if (!hasTyped && value !== '') {
        setHasTyped(true)
      }
    }
  }

  const handleInputBlur = () => {
    // Constrain input value to valid range on blur
    setBidInput((current) => {
      if (current.trim() === '') {
        return current
      }

      const parsed = Number(current)
      if (!Number.isFinite(parsed)) {
        return ''
      }

      const constrained = Math.max(minBid, Math.min(maxBid, parsed))
      return String(constrained)
    })
  }

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()

    if (isSubmitDisabled) {
      return
    }

    if ((parsedBid === null && !hasTyped) || hasValidationIssue) {
      if (!hasTyped) {
        setHasTyped(true)
      }
      setFlashValidation(true)
      return
    }

    const safeBid = parsedBid ?? minBid
    const normalizedBid = Math.min(Math.max(safeBid, minBid), maxBid)
    setBidInput(normalizedBid.toString())
    await bidding.onSubmit(normalizedBid)
  }

  return (
    <section className="flex w-full flex-col gap-4 rounded-3xl border border-success/50 bg-success/10 p-5 text-success-contrast shadow-[0_30px_90px_rgba(56,189,116,0.25)]">
      <header className="flex items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <h2 className="text-sm font-semibold uppercase tracking-[0.4em]">
            {t('title')}
          </h2>
          <p className="text-xs text-success-contrast/80">{t('subtitle')}</p>
        </div>
        <div
          className={`flex shrink-0 items-center gap-2 rounded-lg px-3 py-1.5 ${
            isViewerTurn
              ? 'bg-success/25 border-success/60'
              : 'bg-success/15 border-success/40'
          } border`}
        >
          <span className="text-[10px] font-semibold uppercase tracking-[0.3em] text-success-contrast/80">
            {isViewerTurn ? t('turn.yourTurn') : t('turn.waiting')}
          </span>
          {!isViewerTurn && (
            <span className="text-sm font-bold text-success-contrast/90">
              {activeName}
            </span>
          )}
        </div>
      </header>

      <form
        className="flex flex-col gap-3 rounded-2xl border border-success/30 bg-surface/85 p-4 shadow-inner shadow-success/20"
        onSubmit={handleSubmit}
      >
        <label
          htmlFor="bid-value"
          className="text-xs font-medium uppercase tracking-wide"
        >
          {t('yourBidLabel')}
        </label>
        <div className="flex flex-wrap items-center gap-3">
          <input
            id="bid-value"
            type="text"
            inputMode="numeric"
            pattern="[0-9]*"
            value={bidInput}
            onChange={handleInputChange}
            onBlur={handleInputBlur}
            className={`w-24 rounded-xl border bg-background px-3 py-2 text-sm font-semibold text-foreground outline-none transition disabled:cursor-not-allowed disabled:opacity-60 ${
              hasValidationIssue
                ? 'border-warning/70 focus:border-warning focus:ring focus:ring-warning/30'
                : 'border-success/40 focus:border-success focus:ring focus:ring-success/40'
            } ${flashValidation && hasValidationIssue ? 'animate-pulse' : ''}`}
            disabled={viewerBid !== null || bidding.isPending || !isViewerTurn}
            aria-label={t('bidValueAria')}
            aria-describedby={describedByIds}
            aria-invalid={hasValidationIssue}
            ref={inputRef}
          />
          <button
            type="submit"
            className="rounded-2xl bg-success px-4 py-2 text-sm font-semibold text-success-foreground shadow-lg shadow-success/40 transition hover:bg-success/80 disabled:cursor-not-allowed disabled:bg-success/40 disabled:text-success-foreground/70"
            disabled={isSubmitDisabled}
            aria-label={
              bidding.isPending
                ? t('submit.aria.submitting')
                : parsedBid !== null
                  ? t('submit.aria.submitOf', { bid: parsedBid })
                  : t('submit.aria.submit')
            }
          >
            {bidding.isPending ? t('submit.submitting') : t('submit.label')}
          </button>
        </div>
        <p id="bid-range-hint" className="text-xs text-success-contrast/80">
          {t('allowedRange', { minBid, maxBid })}{' '}
          {isViewerTurn
            ? viewerBid === null
              ? t('hint.chooseAndSubmit')
              : t('hint.submittedWaiting')
            : t('hint.waitingForPlayer', { name: activeName })}
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
        <h3 className="mb-3 text-xs font-semibold uppercase tracking-wide">
          {t('tableBids')}
        </h3>
        <div className="flex flex-col gap-2">
          {seatBids.map(({ seat, name, bid, orientation }) => (
            <div
              key={seat}
              className={`flex items-center justify-between rounded-2xl px-3 py-2 text-sm ${
                seat === phase.to_act
                  ? 'bg-success/20 text-success-contrast'
                  : 'bg-surface text-muted'
              }`}
            >
              <div className="flex flex-col">
                <span className="font-semibold text-foreground">{name}</span>
                <span className="text-[10px] uppercase text-subtle">
                  {orientation}
                </span>
              </div>
              <span className="text-base font-semibold text-foreground">
                {bid ?? '—'}
              </span>
            </div>
          ))}
        </div>
      </div>
    </section>
  )
}
