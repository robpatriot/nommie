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
import type {
  BiddingSnapshot,
  Seat,
  Trump,
  TrumpSelectSnapshot,
} from '@/lib/game-room/types'
import { getPlayerDisplayName } from '@/utils/player-names'
import { getOrientation } from './utils'
import type { GameRoomViewProps } from '../game-room-view'

interface BiddingPanelProps {
  phase: BiddingSnapshot
  viewerSeat: Seat | null
  layoutSeat: Seat
  playerNames: [string, string, string, string]
  bidding: NonNullable<GameRoomViewProps['biddingState']>
  trumpPhase?: TrumpSelectSnapshot
  trump?: GameRoomViewProps['trumpState']
}

export function BiddingPanel({
  phase,
  viewerSeat,
  layoutSeat,
  playerNames,
  bidding,
  trumpPhase,
  trump,
}: BiddingPanelProps) {
  const t = useTranslations('game.gameRoom.bidding')
  const tTrump = useTranslations('game.gameRoom.trumpSelect')
  const tTrumpName = useTranslations('game.gameRoom.trump')
  const tYou = useTranslations('game.gameRoom')
  const isTrumpMode = trumpPhase !== undefined
  const minBid = phase.min_bid
  const maxBid = phase.max_bid
  const handSize = phase.round.hand_size
  const isSpectator = viewerSeat === null
  const viewerBid =
    viewerSeat !== null ? (phase.bids[viewerSeat] ?? null) : null
  const zeroBidLocked = bidding.zeroBidLocked ?? false
  const isViewerTurn =
    !isSpectator &&
    (isTrumpMode && trumpPhase
      ? trumpPhase.to_act === viewerSeat
      : phase.to_act === viewerSeat)
  const activeName = getPlayerDisplayName(
    isTrumpMode && trumpPhase ? trumpPhase.to_act : phase.to_act,
    viewerSeat, // Pass null for spectators so it never matches and shows actual player name
    playerNames,
    tYou('you')
  )
  const [bidInput, setBidInput] = useState<string>('')
  const [flashValidation, setFlashValidation] = useState(false)
  const [hasTyped, setHasTyped] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)
  const [selectedTrump, setSelectedTrump] = useState<Trump | null>(null)

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
    if (isViewerTurn && viewerBid === null && !isTrumpMode) {
      inputRef.current?.focus()
    }
  }, [isViewerTurn, viewerBid, isTrumpMode])

  // Handle trump selection state
  useEffect(() => {
    if (isTrumpMode && trumpPhase) {
      const allowedTrumps = trumpPhase.allowed_trumps
      startTransition(() => {
        if (allowedTrumps.length === 0) {
          setSelectedTrump(null)
          return
        }
        setSelectedTrump((current) => {
          if (current && allowedTrumps.includes(current)) {
            return current
          }
          return null
        })
      })
    }
  }, [isTrumpMode, trumpPhase])

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
        name: getPlayerDisplayName(seat, viewerSeat, playerNames, tYou('you')), // Pass null for spectators
        bid: phase.bids[seat],
        orientation: getOrientation(layoutSeat, seat),
      })),
    [layoutSeat, phase.bids, playerNames, viewerSeat, tYou]
  )

  const { remainingNullBids, sumOfOtherBids } = useMemo(() => {
    const remainingNullBids = phase.bids.filter((bid) => bid === null).length
    const sumOfOtherBids = phase.bids.reduce<number>(
      (total, bid, seatIndex) => {
        if (viewerSeat !== null && seatIndex === viewerSeat) {
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
  // Allow input when not your turn (pre-entry), but disable submit
  const isInputDisabled = viewerBid !== null || bidding.isPending

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

    if (isTrumpMode) {
      if (!selectedTrump || !trump?.canSelect || !trump?.onSelect) {
        return
      }
      await trump.onSelect(selectedTrump)
      return
    }

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

  // Trump selection helpers
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
      case 'NO_TRUMPS':
        return 'NT'
      default:
        return ''
    }
  }

  const getSuitColor = (trump: Trump): string => {
    switch (trump) {
      case 'HEARTS':
      case 'DIAMONDS':
        return 'text-rose-600'
      case 'CLUBS':
      case 'SPADES':
        return 'text-slate-900'
      case 'NO_TRUMPS':
        return 'text-panel-primary-accent'
      default:
        return ''
    }
  }

  const canSelectTrump = trump?.canSelect ?? false
  const isTrumpPending = trump?.isPending ?? false
  const allowedTrumps = isTrumpMode ? (trumpPhase?.allowed_trumps ?? []) : []
  const suits = allowedTrumps.filter((t) => t !== 'NO_TRUMPS').reverse()
  const hasNoTrump = allowedTrumps.includes('NO_TRUMPS')

  // Use panel-primary-accent colors for trump mode
  const borderColor = isTrumpMode
    ? 'border-panel-primary-accent/50'
    : 'border-panel-primary/50'
  const bgColor = isTrumpMode
    ? 'bg-panel-primary-accent/15'
    : 'bg-panel-primary/10'
  const textColor = isTrumpMode
    ? 'text-panel-primary-accent'
    : 'text-panel-primary'

  return (
    <section
      className={`flex w-full flex-col gap-4 rounded-3xl border ${borderColor} ${bgColor} p-5 ${textColor} shadow-elevated`}
    >
      <header className="flex flex-wrap items-start justify-between gap-3">
        {/* min-w-[200px] ensures title/subtitle maintain minimum width before badge wraps */}
        <div className="min-w-[200px] flex-1">
          <h2 className="text-sm font-semibold uppercase tracking-[0.4em]">
            {isTrumpMode ? tTrump('title') : t('title')}
          </h2>
          <p className={`text-xs ${textColor}/80`}>
            {isTrumpMode
              ? isSpectator
                ? tTrump('waitingForPlayerDescription', { name: activeName })
                : isViewerTurn
                  ? tTrump('description')
                  : tTrump('waitingForPlayerDescription', { name: activeName })
              : isSpectator
                ? t('subtitleSpectator', { name: activeName })
                : t('subtitle')}
          </p>
        </div>
        <div
          className={`flex shrink-0 items-center gap-2 rounded-lg px-3 py-1.5 border ${
            isViewerTurn
              ? isTrumpMode
                ? 'bg-panel-primary-accent/25 border-panel-primary-accent/60'
                : 'bg-panel-primary/25 border-panel-primary/60'
              : isTrumpMode
                ? 'bg-panel-primary-accent/15 border-panel-primary-accent/40'
                : 'bg-panel-primary/15 border-panel-primary/40'
          }`}
        >
          <span
            className={`text-[10px] font-semibold uppercase tracking-[0.3em] ${textColor}/80`}
          >
            {isSpectator
              ? isTrumpMode
                ? tTrump('submit.waitingFor', { name: activeName })
                : t('turn.waitingFor', { name: activeName })
              : isViewerTurn
                ? isTrumpMode
                  ? tTrump('submit.confirm')
                  : t('turn.yourTurn')
                : isTrumpMode
                  ? tTrump('submit.waitingFor', { name: activeName })
                  : t('turn.waiting')}
          </span>
          {!isViewerTurn && !isTrumpMode && !isSpectator && (
            <span className={`text-sm font-bold ${textColor}/90`}>
              {activeName}
            </span>
          )}
        </div>
      </header>

      {/* Only show form if viewer is a player (not spectator) and:
          - Bidding mode: user hasn't submitted their bid yet (viewerBid === null)
          - Trump mode: it's the viewer's turn to select trump (canSelectTrump) */}
      {!isSpectator && (isTrumpMode ? canSelectTrump : viewerBid === null) && (
        <form
          className={`flex flex-col gap-3 rounded-2xl border bg-surface/85 p-4 shadow-inner ${
            isTrumpMode
              ? 'border-panel-primary-accent/30 shadow-panel-primary-accent/20'
              : 'border-panel-primary/30 shadow-panel-primary/20'
          }`}
          onSubmit={handleSubmit}
        >
          {isTrumpMode ? (
            <>
              <div className="flex flex-col gap-3">
                {suits.length > 0 && (
                  <div
                    className="grid gap-2"
                    style={{
                      gridTemplateColumns: `repeat(${suits.length}, minmax(0, 1fr))`,
                    }}
                  >
                    {suits.map((option) => {
                      const isSelected = option === selectedTrump
                      const disabled = !canSelectTrump || isTrumpPending
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
                              ? 'border-panel-primary-accent bg-panel-primary-accent/30 text-panel-primary-accent shadow-md shadow-panel-primary-accent/30'
                              : canSelectTrump
                                ? 'border-panel-primary-accent/40 bg-surface text-panel-primary-accent hover:border-panel-primary-accent hover:bg-panel-primary-accent/15'
                                : 'border-border bg-surface text-muted'
                          } ${
                            disabled
                              ? 'cursor-not-allowed opacity-60'
                              : 'cursor-pointer'
                          }`}
                          aria-label={
                            isSelected
                              ? tTrump('optionAriaSelected', {
                                  trump: tTrumpName(option),
                                })
                              : tTrump('optionAria', {
                                  trump: tTrumpName(option),
                                })
                          }
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

                {hasNoTrump && (
                  <div
                    className="grid gap-2"
                    style={{ gridTemplateColumns: '1fr' }}
                  >
                    <button
                      type="button"
                      onClick={() => {
                        if (!canSelectTrump || isTrumpPending) {
                          return
                        }
                        setSelectedTrump(
                          selectedTrump === 'NO_TRUMPS' ? null : 'NO_TRUMPS'
                        )
                      }}
                      disabled={!canSelectTrump || isTrumpPending}
                      className={`flex items-center justify-center rounded-2xl border px-4 py-3 text-center transition ${
                        selectedTrump === 'NO_TRUMPS'
                          ? 'border-panel-primary-accent bg-panel-primary-accent/30 text-panel-primary-accent shadow-md shadow-panel-primary-accent/30'
                          : canSelectTrump
                            ? 'border-panel-primary-accent/40 bg-surface text-panel-primary-accent hover:border-panel-primary-accent hover:bg-panel-primary-accent/15'
                            : 'border-border bg-surface text-muted'
                      } ${
                        !canSelectTrump || isTrumpPending
                          ? 'cursor-not-allowed opacity-60'
                          : 'cursor-pointer'
                      }`}
                      aria-label={
                        selectedTrump === 'NO_TRUMPS'
                          ? tTrump('optionAriaSelected', {
                              trump: tTrumpName('NO_TRUMPS'),
                            })
                          : tTrump('optionAria', {
                              trump: tTrumpName('NO_TRUMPS'),
                            })
                      }
                      aria-pressed={selectedTrump === 'NO_TRUMPS'}
                    >
                      <span className="text-xl font-semibold text-panel-primary-accent">
                        {tTrumpName('NO_TRUMPS')}
                      </span>
                    </button>
                  </div>
                )}
              </div>

              <button
                type="submit"
                className="w-full rounded-2xl bg-panel-primary-accent px-4 py-3 text-sm font-semibold text-primary-foreground shadow-lg shadow-panel-primary-accent/40 transition hover:bg-panel-primary-accent/80 disabled:cursor-not-allowed disabled:bg-panel-primary-accent/40 disabled:text-primary-foreground/70 dark:text-accent-foreground dark:disabled:text-accent-foreground/70"
                disabled={!canSelectTrump || isTrumpPending || !selectedTrump}
                aria-label={
                  isTrumpPending
                    ? tTrump('submit.aria.selecting')
                    : selectedTrump
                      ? tTrump('submit.aria.confirm', {
                          trump: tTrumpName(selectedTrump),
                        })
                      : tTrump('submit.aria.select')
                }
              >
                {isTrumpPending
                  ? tTrump('submit.choosing')
                  : canSelectTrump
                    ? tTrump('submit.confirm')
                    : tTrump('submit.waitingFor', { name: activeName })}
              </button>

              <p className={`text-xs ${textColor}/75`}>
                {canSelectTrump
                  ? tTrump('hint.canSelect')
                  : tTrump('hint.waitingForPlayer', { name: activeName })}
              </p>
            </>
          ) : (
            <>
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
                      : 'border-panel-primary/40 focus:border-panel-primary focus:ring focus:ring-panel-primary/40'
                  } ${flashValidation && hasValidationIssue ? 'animate-pulse' : ''}`}
                  disabled={isInputDisabled}
                  aria-label={t('bidValueAria')}
                  aria-describedby={describedByIds}
                  aria-invalid={hasValidationIssue}
                  ref={inputRef}
                />
                <button
                  type="submit"
                  className="rounded-2xl bg-panel-primary px-4 py-2 text-sm font-semibold text-primary-foreground shadow-lg shadow-panel-primary/40 transition hover:bg-panel-primary/80 disabled:cursor-not-allowed disabled:bg-panel-primary/40 disabled:text-primary-foreground/70 dark:text-success-foreground dark:disabled:text-success-foreground/70"
                  disabled={isSubmitDisabled}
                  aria-label={
                    bidding.isPending
                      ? t('submit.aria.submitting')
                      : parsedBid !== null
                        ? t('submit.aria.submitOf', { bid: parsedBid })
                        : t('submit.aria.submit')
                  }
                >
                  {bidding.isPending
                    ? t('submit.submitting')
                    : t('submit.label')}
                </button>
              </div>
              <p id="bid-range-hint" className={`text-xs ${textColor}/80`}>
                {t('allowedRange', { minBid, maxBid })}{' '}
                {isViewerTurn
                  ? viewerBid === null
                    ? t('hint.chooseAndSubmit')
                    : t('hint.submittedWaiting')
                  : viewerBid === null
                    ? t('hint.waitingForPlayer', { name: activeName })
                    : t('hint.submittedWaiting')}
                {!isViewerTurn && viewerBid === null && (
                  <span className="block mt-1 text-panel-primary/70">
                    {t('hint.preEntry')}
                  </span>
                )}
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
            </>
          )}
        </form>
      )}

      <div
        className={`rounded-2xl border bg-surface/70 p-4 ${
          isTrumpMode
            ? 'border-panel-primary-accent/20'
            : 'border-panel-primary/20'
        }`}
      >
        <h3 className="mb-3 text-xs font-semibold uppercase tracking-wide">
          {t('tableBids')}
        </h3>
        <div className="flex flex-col gap-2">
          {seatBids.map(({ seat, name, bid, orientation }) => {
            const isActiveInCurrentPhase =
              isTrumpMode && trumpPhase
                ? seat === trumpPhase.to_act
                : seat === phase.to_act
            return (
              <div
                key={seat}
                className={`flex items-center justify-between rounded-2xl px-3 py-2 text-sm ${
                  isActiveInCurrentPhase
                    ? isTrumpMode
                      ? 'bg-panel-primary-accent/20 text-panel-primary-accent'
                      : 'bg-panel-primary/20 text-panel-primary'
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
            )
          })}
        </div>
      </div>
    </section>
  )
}
