'use client'

import {
  type FormEvent,
  useCallback,
  useEffect,
  useMemo,
  useState,
} from 'react'
import Link from 'next/link'

import type {
  Card,
  BiddingSnapshot,
  GameSnapshot,
  PhaseSnapshot,
  RoundPublic,
  Seat,
  Trump,
  TrumpSelectSnapshot,
  TrickSnapshot,
} from '@/lib/game-room/types'

export interface AiSeatSelection {
  registryName: string
  registryVersion?: string
  seed?: number
}

export interface GameRoomViewProps {
  gameId: number
  snapshot: GameSnapshot
  playerNames: [string, string, string, string]
  viewerSeat?: Seat
  viewerHand?: Card[]
  status: {
    lastSyncedAt: string
    isPolling: boolean
  }
  onRefresh?: () => void
  isRefreshing?: boolean
  error?: {
    message: string
    traceId?: string
  } | null
  readyState?: {
    canReady: boolean
    isPending: boolean
    hasMarked: boolean
    onReady: () => void
  }
  biddingState?: {
    viewerSeat: Seat
    isPending: boolean
    onSubmit: (bid: number) => Promise<void> | void
  }
  trumpState?: {
    viewerSeat: Seat
    toAct: Seat
    allowedTrumps: Trump[]
    canSelect: boolean
    isPending: boolean
    onSelect?: (trump: Trump) => Promise<void> | void
  }
  playState?: {
    viewerSeat: Seat
    playable: Card[]
    isPending: boolean
    onPlay: (card: Card) => Promise<void> | void
  }
  aiSeatState?: {
    totalSeats: number
    availableSeats: number
    aiSeats: number
    isPending: boolean
    canAdd: boolean
    canRemove: boolean
    onAdd: (selection?: AiSeatSelection) => Promise<void> | void
    onRemove?: () => Promise<void> | void
    onRemoveSeat?: (seat: Seat) => Promise<void> | void
    onUpdateSeat?: (
      seat: Seat,
      selection: AiSeatSelection
    ) => Promise<void> | void
    registry?: {
      entries: Array<{
        name: string
        version: string
      }>
      isLoading: boolean
      error?: string | null
      defaultName?: string
    }
    seats: Array<{
      seat: Seat
      name: string
      userId: number | null
      isOccupied: boolean
      isAi: boolean
      isReady: boolean
      aiProfile?: {
        name: string
        version: string
      } | null
    }>
  }
}

export function GameRoomView(props: GameRoomViewProps) {
  const {
    snapshot,
    playerNames,
    viewerSeat = 0,
    viewerHand = [],
    status,
    gameId,
    onRefresh,
    isRefreshing = false,
    error,
    readyState,
    biddingState,
    trumpState,
    playState,
    aiSeatState,
  } = props
  const phase = snapshot.phase
  const round = getRound(phase)
  const activeSeat = getActiveSeat(phase)
  const seatDisplayName = useCallback(
    (seat: Seat) => (seat === viewerSeat ? 'You' : playerNames[seat]),
    [playerNames, viewerSeat]
  )
  const activeName =
    typeof activeSeat === 'number' ? seatDisplayName(activeSeat) : 'Waiting'
  const syncLabel = new Date(status.lastSyncedAt).toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
  })

  const trickMap = getCurrentTrickMap(phase)
  const seatSummaries = buildSeatSummaries({
    playerNames,
    viewerSeat,
    phase,
    scores: snapshot.game.scores_total,
    trickMap,
    round,
    activeSeat,
  })

  const [selectedCard, setSelectedCard] = useState<Card | null>(null)

  useEffect(() => {
    if (phase.phase !== 'Trick' || !playState) {
      setSelectedCard(null)
      return
    }

    if (selectedCard && !playState.playable.includes(selectedCard)) {
      setSelectedCard(null)
    }
  }, [phase, playState, selectedCard])

  const handlePlayCard = useCallback(
    async (card: Card) => {
      if (!playState) {
        return
      }
      await playState.onPlay(card)
      setSelectedCard(null)
    },
    [playState]
  )

  return (
    <div className="flex min-h-screen flex-col bg-slate-950 text-slate-100">
      <header className="border-b border-slate-800 bg-slate-900/70">
        <div className="mx-auto flex w-full max-w-7xl flex-wrap items-center justify-between gap-2 px-4 py-4 sm:px-6 lg:px-10">
          <div className="flex flex-1 flex-col gap-1">
            <span className="text-sm font-medium text-slate-400">
              Game #{gameId}
            </span>
            <h1 className="text-2xl font-semibold text-white">Nommie Table</h1>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            {onRefresh ? (
              <button
                type="button"
                onClick={onRefresh}
                className="rounded-md border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-200 transition hover:border-slate-500 hover:text-white"
                disabled={isRefreshing}
              >
                {isRefreshing ? 'Refreshing…' : 'Refresh'}
              </button>
            ) : null}
            <button
              type="button"
              className="rounded-md border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-200 transition hover:border-slate-500 hover:text-white"
            >
              Copy Invite Link
            </button>
            <Link
              href="/lobby"
              className="rounded-md bg-slate-100 px-3 py-1.5 text-sm font-semibold text-slate-900 transition hover:bg-white"
            >
              Back to Lobby
            </Link>
          </div>
        </div>
      </header>

      <main className="flex flex-1 flex-col gap-6 px-4 py-6 sm:px-6 lg:px-10">
        <section className="flex flex-col gap-4 rounded-xl border border-slate-800 bg-slate-900/60 p-4 shadow-lg shadow-slate-900/30">
          <div className="flex flex-wrap items-center justify-between gap-4">
            <div>
              <p className="text-sm uppercase tracking-wide text-slate-400">
                Phase
              </p>
              <div className="text-2xl font-semibold text-white">
                {getPhaseLabel(phase)}
              </div>
            </div>
            <div className="flex items-center gap-3 text-sm text-slate-300">
              <span className="flex items-center gap-2">
                <span
                  className={`inline-flex h-2.5 w-2.5 items-center justify-center rounded-full ${
                    status.isPolling
                      ? 'animate-pulse bg-emerald-400'
                      : 'bg-slate-500'
                  }`}
                  aria-hidden
                />
                {status.isPolling ? 'Syncing…' : 'Idle'}
              </span>
              <span aria-live="off" className="text-slate-500">
                Last synced {syncLabel}
              </span>
            </div>
          </div>
          {error ? (
            <div className="rounded-lg border border-amber-400 bg-amber-500/10 px-3 py-2 text-sm text-amber-200">
              <p>{error.message}</p>
              {error.traceId ? (
                <p className="text-xs text-amber-300/80">
                  traceId: {error.traceId}
                </p>
              ) : null}
            </div>
          ) : null}
          {round ? (
            <div className="grid gap-3 text-sm text-slate-300 sm:grid-cols-4">
              <PhaseFact label="Round" value={`#${snapshot.game.round_no}`} />
              <PhaseFact label="Hand Size" value={round.hand_size.toString()} />
              <PhaseFact
                label="Dealer"
                value={seatDisplayName(snapshot.game.dealer)}
              />
              <PhaseFact label="Trump" value={formatTrump(round.trump)} />
            </div>
          ) : null}
          <div className="flex flex-wrap items-center gap-4 text-sm text-slate-200">
            <span className="rounded-full bg-slate-800 px-3 py-1 font-medium">
              Turn: {activeName}
            </span>
            {phase.phase === 'Trick' ? (
              <span className="text-slate-400">
                Trick {phase.data.trick_no} of {round?.hand_size ?? '?'}
              </span>
            ) : null}
          </div>
        </section>

        <section className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_320px] xl:grid-cols-[minmax(0,1fr)_360px]">
          <div className="flex flex-col gap-6">
            <div className="relative mx-auto grid h-full w-full max-w-4xl grid-cols-3 grid-rows-3 gap-4">
              {seatSummaries.map((summary) => (
                <SeatCard key={summary.seat} summary={summary} />
              ))}
              <TrickArea
                trickMap={trickMap}
                getSeatName={seatDisplayName}
                round={round}
                phase={phase}
                viewerSeat={viewerSeat}
              />
            </div>

            <PlayerHand
              viewerHand={viewerHand}
              phase={phase}
              playerNames={playerNames}
              viewerSeat={viewerSeat}
              playState={playState}
              selectedCard={selectedCard}
              onSelectCard={setSelectedCard}
            />

            <PlayerActions
              phase={phase}
              viewerSeat={viewerSeat}
              playerNames={playerNames}
              bidding={biddingState}
              trump={trumpState}
              play={playState}
              selectedCard={selectedCard}
              onPlayCard={handlePlayCard}
            />
          </div>

          <ScoreSidebar
            playerNames={playerNames}
            scores={snapshot.game.scores_total}
            round={round}
            readyState={readyState}
            aiState={aiSeatState}
          />
        </section>
      </main>
    </div>
  )
}

interface SeatSummary {
  seat: Seat
  orientation: 'top' | 'left' | 'right' | 'bottom'
  name: string
  score: number
  isViewer: boolean
  isActive: boolean
  tricksWon?: number
  currentCard?: Card
  bid?: number | null
}

function buildSeatSummaries(params: {
  playerNames: [string, string, string, string]
  viewerSeat: Seat
  phase: PhaseSnapshot
  scores: [number, number, number, number]
  trickMap: Map<Seat, Card>
  round: RoundPublic | null
  activeSeat: Seat | null
}): SeatSummary[] {
  const {
    playerNames,
    viewerSeat,
    phase,
    scores,
    trickMap,
    round,
    activeSeat,
  } = params

  return [0, 1, 2, 3].map((seat) => {
    const orientation = getOrientation(viewerSeat, seat as Seat)
    const isViewer = seat === viewerSeat
    const tricksWon = round?.tricks_won[seat as Seat]
    const currentCard = trickMap.get(seat as Seat)
    const bid = getBidForSeat(phase, seat as Seat)
    const isActive = activeSeat === seat

    return {
      seat: seat as Seat,
      orientation,
      name: playerNames[seat as Seat],
      score: scores[seat as Seat],
      isViewer,
      tricksWon,
      currentCard,
      bid,
      isActive,
    }
  })
}

function SeatCard({ summary }: { summary: SeatSummary }) {
  const {
    orientation,
    name,
    score,
    isViewer,
    tricksWon,
    currentCard,
    bid,
    isActive,
  } = summary

  const positionStyles: Record<SeatSummary['orientation'], string> = {
    top: 'col-start-2 row-start-1 justify-self-center',
    left: 'col-start-1 row-start-2 justify-self-start',
    right: 'col-start-3 row-start-2 justify-self-end',
    bottom: 'col-start-2 row-start-3 justify-self-center',
  }

  return (
    <div
      className={`flex w-full max-w-[220px] flex-col gap-2 rounded-xl border border-slate-800 bg-slate-900/70 p-3 text-center shadow-sm ${
        isActive
          ? 'ring-2 ring-emerald-400 ring-offset-2 ring-offset-slate-950'
          : ''
      } ${positionStyles[orientation]}`}
    >
      <div className="flex flex-col gap-1">
        <span className="text-xs uppercase tracking-wide text-slate-500">
          {orientation === 'bottom' ? 'You' : 'Player'}
        </span>
        <span className="text-lg font-semibold text-white">{name}</span>
        <span className="text-xs text-slate-400">Score {score}</span>
      </div>
      <div className="flex items-center justify-center gap-3 text-xs text-slate-300">
        {typeof tricksWon === 'number' ? (
          <span className="rounded-full bg-slate-800 px-2 py-1 font-medium">
            Tricks {tricksWon}
          </span>
        ) : null}
        {bid !== undefined ? (
          <span className="rounded-full border border-slate-800 px-2 py-1 font-medium">
            Bid {bid ?? '—'}
          </span>
        ) : null}
        {currentCard ? (
          <span className="rounded-md bg-slate-800 px-2 py-1 font-semibold tracking-wide text-white">
            {currentCard}
          </span>
        ) : null}
      </div>
      {isViewer ? (
        <span className="self-center rounded-full bg-emerald-500/20 px-3 py-1 text-xs font-semibold text-emerald-300">
          You
        </span>
      ) : null}
    </div>
  )
}

function TrickArea({
  trickMap,
  getSeatName,
  round,
  phase,
  viewerSeat,
}: {
  trickMap: Map<Seat, Card>
  getSeatName: (seat: Seat) => string
  round: RoundPublic | null
  phase: PhaseSnapshot
  viewerSeat: Seat
}) {
  const cards = Array.from(trickMap.entries()).map(([seat, card]) => ({
    seat,
    card,
    label: getSeatName(seat),
    orientation: getOrientation(viewerSeat, seat),
  }))

  return (
    <div className="col-start-2 row-start-2 flex h-64 flex-col items-center justify-center gap-4 rounded-2xl border border-slate-800 bg-slate-900/70 p-6">
      <p className="text-sm uppercase tracking-wide text-slate-500">
        Current Trick
      </p>
      <div className="flex flex-wrap items-center justify-center gap-6">
        {cards.length === 0 ? (
          <span className="text-sm text-slate-500">Waiting for lead…</span>
        ) : (
          cards.map(({ seat, card, label, orientation }) => (
            <div key={seat} className="flex flex-col items-center gap-2">
              <span className="text-xs uppercase tracking-wide text-slate-500">
                {label}
              </span>
              <span className="rounded-xl bg-slate-800 px-3 py-2 text-lg font-semibold tracking-wider text-white">
                {card}
              </span>
              <span className="text-[10px] uppercase text-slate-500">
                {orientation}
              </span>
            </div>
          ))
        )}
      </div>
      {phase.phase === 'Trick' ? (
        <p className="text-xs text-slate-400">
          Leader: {getSeatName(phase.data.leader)} — Trick {phase.data.trick_no}{' '}
          of {round?.hand_size ?? '?'}
        </p>
      ) : null}
    </div>
  )
}

function PlayerHand({
  viewerHand,
  phase,
  playerNames,
  viewerSeat,
  playState,
  selectedCard,
  onSelectCard,
}: {
  viewerHand: Card[]
  phase: PhaseSnapshot
  playerNames: [string, string, string, string]
  viewerSeat: Seat
  playState?: GameRoomViewProps['playState']
  selectedCard: Card | null
  onSelectCard: (card: Card | null) => void
}) {
  const isTrickPhase = phase.phase === 'Trick' && !!playState
  const viewerTurn =
    isTrickPhase &&
    playState &&
    phase.phase === 'Trick' &&
    phase.data.to_act === playState.viewerSeat
  const playableCards = useMemo(
    () => new Set(playState?.playable ?? []),
    [playState]
  )
  const waitingOnSeat = phase.phase === 'Trick' ? phase.data.to_act : null
  const waitingOnName =
    waitingOnSeat === null
      ? null
      : waitingOnSeat === viewerSeat
        ? 'You'
        : playerNames[waitingOnSeat]

  let handStatus = 'Read-only preview'

  if (!viewerHand.length) {
    handStatus = 'Hand will appear once the game starts.'
  } else if (isTrickPhase) {
    if (!viewerTurn) {
      handStatus = `Waiting for ${waitingOnName ?? 'next player'} to play`
    } else if (playState?.isPending) {
      handStatus = 'Playing card…'
    } else if (selectedCard) {
      handStatus = `Selected ${selectedCard}`
    } else {
      handStatus = 'Select a card to play'
    }
  }

  const handleCardClick = (card: Card) => {
    if (!isTrickPhase || !playState) {
      return
    }

    const isPlayable = playableCards.has(card)
    if (!viewerTurn || !isPlayable || playState.isPending) {
      return
    }

    onSelectCard(selectedCard === card ? null : card)
  }

  return (
    <section className="mx-auto flex w-full max-w-4xl flex-col gap-3 rounded-2xl border border-slate-800 bg-slate-900/70 p-4">
      <header className="flex items-center justify-between">
        <h2 className="text-sm uppercase tracking-wide text-slate-400">
          Your Hand
        </h2>
        <span className="text-xs text-slate-500">{handStatus}</span>
      </header>
      <div className="flex flex-wrap justify-center gap-2">
        {viewerHand.length === 0 ? (
          <span className="text-sm text-slate-500">
            Hand will appear once available.
          </span>
        ) : (
          viewerHand.map((card) => {
            const isPlayable = playableCards.has(card)
            const isSelected = selectedCard === card
            const isDisabled =
              !isTrickPhase ||
              !playState ||
              !isPlayable ||
              !viewerTurn ||
              playState.isPending

            return (
              <button
                key={card}
                type="button"
                onClick={() => handleCardClick(card)}
                disabled={isDisabled}
                className={`rounded-xl border px-3 py-2 text-lg font-semibold tracking-wide transition ${
                  isSelected
                    ? 'border-emerald-400 bg-emerald-500/20 text-white shadow-lg shadow-emerald-500/30'
                    : isPlayable && viewerTurn
                      ? 'border-emerald-500/60 bg-slate-800 text-white hover:border-emerald-300 hover:bg-emerald-500/10'
                      : 'border-slate-700 bg-slate-800 text-slate-400'
                } ${
                  isDisabled
                    ? 'cursor-not-allowed opacity-60'
                    : 'cursor-pointer'
                }`}
                aria-pressed={isSelected}
              >
                {card}
              </button>
            )
          })
        )}
      </div>
    </section>
  )
}

function PlayerActions({
  phase,
  viewerSeat,
  playerNames,
  bidding,
  trump,
  play,
  selectedCard,
  onPlayCard,
}: {
  phase: PhaseSnapshot
  viewerSeat: Seat
  playerNames: [string, string, string, string]
  bidding?: GameRoomViewProps['biddingState']
  trump?: GameRoomViewProps['trumpState']
  play?: GameRoomViewProps['playState']
  selectedCard: Card | null
  onPlayCard: (card: Card) => Promise<void> | void
}) {
  if (phase.phase === 'Bidding' && bidding) {
    return (
      <BiddingPanel
        phase={phase.data}
        viewerSeat={bidding.viewerSeat}
        layoutSeat={viewerSeat}
        playerNames={playerNames}
        bidding={bidding}
      />
    )
  }

  if (phase.phase === 'TrumpSelect') {
    return (
      <TrumpSelectPanel
        phase={phase.data}
        viewerSeat={viewerSeat}
        playerNames={playerNames}
        trump={trump}
      />
    )
  }

  if (phase.phase === 'Trick' && play) {
    return (
      <PlayPanel
        phase={phase.data}
        playerNames={playerNames}
        play={play}
        selectedCard={selectedCard}
        onPlayCard={onPlayCard}
      />
    )
  }

  return (
    <section className="mx-auto flex w-full max-w-4xl flex-col gap-3 rounded-2xl border border-slate-800 bg-slate-900/50 p-4 text-sm text-slate-300">
      <header className="flex items-center justify-between">
        <h2 className="text-sm font-semibold uppercase tracking-wide text-slate-400">
          Table Actions
        </h2>
        <span className="text-xs text-slate-500">Interactive controls</span>
      </header>
      <p>
        No interactive controls are available for the current phase. They will
        appear here when required.
      </p>
    </section>
  )
}

function BiddingPanel({
  phase,
  viewerSeat,
  layoutSeat,
  playerNames,
  bidding,
}: {
  phase: BiddingSnapshot
  viewerSeat: Seat
  layoutSeat: Seat
  playerNames: [string, string, string, string]
  bidding: NonNullable<GameRoomViewProps['biddingState']>
}) {
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
    <section className="mx-auto flex w-full max-w-4xl flex-col gap-4 rounded-2xl border border-emerald-500/30 bg-emerald-950/30 p-4">
      <header className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <h2 className="text-sm font-semibold uppercase tracking-wide text-emerald-200">
            Bidding
          </h2>
          <p className="text-xs text-emerald-100/80">
            Select your bid between {minBid} and {maxBid}. Once submitted, the
            next player will be prompted automatically.
          </p>
        </div>
        <div className="rounded-full border border-emerald-500/40 bg-emerald-500/10 px-3 py-1 text-xs font-medium text-emerald-200">
          Waiting on: {activeName}
        </div>
      </header>

      <form
        className="flex flex-col gap-3 rounded-lg border border-emerald-500/20 bg-slate-900/60 p-4 shadow-inner shadow-emerald-900/30"
        onSubmit={handleSubmit}
      >
        <label
          htmlFor="bid-value"
          className="text-xs font-medium uppercase tracking-wide text-emerald-200"
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
            className="w-24 rounded-md border border-emerald-500/30 bg-slate-950 px-3 py-2 text-sm font-semibold text-emerald-100 outline-none transition focus:border-emerald-300 focus:ring focus:ring-emerald-400/40 disabled:cursor-not-allowed disabled:opacity-60"
            disabled={viewerBid !== null || bidding.isPending || !isViewerTurn}
            aria-describedby="bid-range-hint"
          />
          <button
            type="submit"
            className="rounded-md bg-emerald-500 px-4 py-2 text-sm font-semibold text-slate-900 transition hover:bg-emerald-400 disabled:cursor-not-allowed disabled:bg-emerald-500/40 disabled:text-slate-700"
            disabled={isSubmitDisabled}
          >
            {bidding.isPending ? 'Submitting…' : 'Submit Bid'}
          </button>
        </div>
        <p id="bid-range-hint" className="text-xs text-emerald-100/80">
          Allowed range: {minBid} – {maxBid}.{' '}
          {isViewerTurn
            ? viewerBid === null
              ? 'Choose a value and submit before time runs out.'
              : 'Bid submitted — waiting for other players.'
            : `Waiting for ${activeName} to bid.`}
        </p>
      </form>

      <div className="rounded-lg border border-emerald-500/10 bg-slate-900/60 p-4">
        <h3 className="mb-3 text-xs font-semibold uppercase tracking-wide text-emerald-200">
          Bid Tracker
        </h3>
        <ul className="grid gap-2 sm:grid-cols-2">
          {seatBids.map(({ seat, name, bid, orientation }) => (
            <li
              key={seat}
              className={`flex items-center justify-between rounded-md border border-slate-800/80 bg-slate-900/60 px-3 py-2 text-sm ${
                seat === phase.to_act
                  ? 'border-emerald-500/60 bg-emerald-500/10 text-emerald-100'
                  : ''
              }`}
            >
              <div className="flex flex-col">
                <span className="font-medium text-white">{name}</span>
                <span className="text-[10px] uppercase text-slate-500">
                  {orientation}
                </span>
              </div>
              <span className="text-sm font-semibold text-slate-200">
                {bid ?? '—'}
              </span>
            </li>
          ))}
        </ul>
      </div>
    </section>
  )
}

function TrumpSelectPanel({
  phase,
  viewerSeat,
  playerNames,
  trump,
}: {
  phase: TrumpSelectSnapshot
  viewerSeat: Seat
  playerNames: [string, string, string, string]
  trump?: GameRoomViewProps['trumpState']
}) {
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
    <section className="mx-auto flex w-full max-w-4xl flex-col gap-4 rounded-2xl border border-purple-500/40 bg-purple-500/10 p-4">
      <header className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <h2 className="text-sm font-semibold uppercase tracking-wide text-purple-200">
            Select Trump
          </h2>
          <p className="text-xs text-purple-100/80">
            Choose the trump suit for this round. Trump cards outrank all other
            suits.
          </p>
        </div>
        <div className="rounded-full border border-purple-400/40 bg-purple-400/10 px-3 py-1 text-xs font-medium text-purple-100">
          Waiting on: {activeName}
        </div>
      </header>

      <form
        className="flex flex-col gap-3 rounded-lg border border-purple-400/20 bg-slate-900/60 p-4 shadow-inner shadow-purple-900/30"
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
                    ? 'border-purple-300 bg-purple-500/30 text-white shadow-lg shadow-purple-500/30'
                    : canSelect
                      ? 'border-purple-400/40 bg-slate-800 text-purple-100 hover:border-purple-300 hover:bg-purple-500/10'
                      : 'border-slate-800 bg-slate-900 text-slate-400'
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
          className="w-full rounded-md bg-purple-400 px-4 py-2 text-sm font-semibold text-slate-900 transition hover:bg-purple-300 disabled:cursor-not-allowed disabled:bg-purple-500/40 disabled:text-slate-600"
          disabled={!canSelect || isPending || !selectedTrump}
        >
          {submitLabel}
        </button>

        <p className="text-xs text-purple-100/70">
          {canSelect
            ? 'Select a trump suit and confirm to continue to trick play.'
            : `Waiting for ${activeName} to choose the trump suit.`}
        </p>
      </form>
    </section>
  )
}

function PlayPanel({
  phase,
  playerNames,
  play,
  selectedCard,
  onPlayCard,
}: {
  phase: TrickSnapshot
  playerNames: [string, string, string, string]
  play: NonNullable<GameRoomViewProps['playState']>
  selectedCard: Card | null
  onPlayCard: (card: Card) => Promise<void> | void
}) {
  const isViewerTurn = phase.to_act === play.viewerSeat
  const activeName =
    phase.to_act === play.viewerSeat ? 'You' : playerNames[phase.to_act]
  const isCardPlayable = !!selectedCard && play.playable.includes(selectedCard)
  const isSubmitDisabled = !isViewerTurn || play.isPending || !isCardPlayable

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()

    if (isSubmitDisabled || !selectedCard) {
      return
    }

    await onPlayCard(selectedCard)
  }

  return (
    <section className="mx-auto flex w-full max-w-4xl flex-col gap-4 rounded-2xl border border-indigo-500/40 bg-indigo-500/10 p-4">
      <header className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <h2 className="text-sm font-semibold uppercase tracking-wide text-indigo-200">
            Play Card
          </h2>
          <p className="text-xs text-indigo-100/80">
            Choose a legal card from your hand. Only legal cards are enabled.
          </p>
        </div>
        <div className="rounded-full border border-indigo-500/40 bg-indigo-500/10 px-3 py-1 text-xs font-medium text-indigo-200">
          Waiting on: {activeName}
        </div>
      </header>

      <form
        className="flex flex-col gap-3 rounded-lg border border-indigo-500/20 bg-slate-900/60 p-4 shadow-inner shadow-indigo-900/30"
        onSubmit={handleSubmit}
      >
        <div className="flex flex-wrap items-center gap-3 text-sm text-indigo-100">
          <span className="text-xs uppercase tracking-wide text-indigo-300">
            Selected Card
          </span>
          <span className="rounded-md border border-indigo-500/40 bg-slate-900/80 px-3 py-1 font-semibold text-white">
            {selectedCard ?? '—'}
          </span>
        </div>
        <button
          type="submit"
          className="w-full rounded-md bg-indigo-400 px-4 py-2 text-sm font-semibold text-slate-900 transition hover:bg-indigo-300 disabled:cursor-not-allowed disabled:bg-indigo-500/40 disabled:text-slate-600"
          disabled={isSubmitDisabled}
        >
          {play.isPending
            ? 'Playing…'
            : isViewerTurn
              ? 'Play Selected Card'
              : `Waiting for ${activeName}`}
        </button>
        <p className="text-xs text-indigo-100/80">
          Legal cards: {play.playable.length ? play.playable.join(', ') : '—'}
        </p>
      </form>
    </section>
  )
}

function ScoreSidebar({
  playerNames,
  scores,
  round,
  readyState,
  aiState,
}: {
  playerNames: [string, string, string, string]
  scores: [number, number, number, number]
  round: RoundPublic | null
  readyState?: GameRoomViewProps['readyState']
  aiState?: GameRoomViewProps['aiSeatState']
}) {
  return (
    <aside className="flex h-full flex-col gap-4 rounded-2xl border border-slate-800 bg-slate-900/70 p-4">
      <header className="flex items-center justify-between">
        <h2 className="text-base font-semibold text-white">Scores</h2>
        <span className="text-xs text-slate-500">Updated each sync</span>
      </header>

      <details
        className="rounded-xl border border-slate-800 bg-slate-900/40"
        open
      >
        <summary className="cursor-pointer list-none rounded-xl px-4 py-3 text-sm font-medium text-slate-200 transition hover:bg-slate-800/80">
          Cumulative Totals
        </summary>
        <div className="px-4 pb-3">
          <ul className="flex flex-col gap-2 text-sm text-slate-300">
            {scores.map((score, idx) => (
              <li
                key={playerNames[idx]}
                className="flex items-center justify-between"
              >
                <span>{playerNames[idx]}</span>
                <span className="font-semibold text-white">{score}</span>
              </li>
            ))}
          </ul>
        </div>
      </details>

      {round ? (
        <div className="rounded-xl border border-slate-800 bg-slate-900/40 p-4 text-sm text-slate-300">
          <div className="mb-2 text-xs uppercase tracking-wide text-slate-500">
            Round Snapshot
          </div>
          <p>Hand size: {round.hand_size}</p>
          <p>Trump: {formatTrump(round.trump)}</p>
          <p>Tricks won: {round.tricks_won.join(' / ')}</p>
        </div>
      ) : null}

      <ReadyPanel readyState={readyState} />
      <AiSeatManager aiState={aiState} />
    </aside>
  )
}

function ReadyPanel({
  readyState,
}: {
  readyState?: GameRoomViewProps['readyState']
}) {
  if (!readyState) {
    return (
      <div className="rounded-xl border border-dashed border-slate-800 bg-slate-900/40 p-4 text-xs text-slate-500">
        Ready controls will appear once interactions are available.
      </div>
    )
  }

  if (!readyState.canReady) {
    return (
      <div className="rounded-xl border border-slate-800 bg-slate-900/40 p-4 text-sm text-slate-300">
        <h3 className="mb-1 text-sm font-semibold text-white">Game in play</h3>
        <p>The table is active. Actions will surface here when required.</p>
      </div>
    )
  }

  return (
    <div className="rounded-xl border border-emerald-500/40 bg-emerald-500/10 p-4 text-sm text-emerald-100">
      <h3 className="mb-2 text-sm font-semibold text-emerald-200">Ready Up</h3>
      <p className="mb-3 text-xs text-emerald-200/80">
        Mark yourself ready. The game auto-starts when all four seats are ready.
      </p>
      <button
        type="button"
        onClick={() => readyState.onReady()}
        className="w-full rounded-md bg-emerald-500 px-3 py-2 text-sm font-semibold text-slate-900 transition hover:bg-emerald-400 disabled:cursor-not-allowed disabled:bg-emerald-500/60 disabled:text-slate-800"
        disabled={readyState.isPending || readyState.hasMarked}
      >
        {readyState.isPending
          ? 'Marking…'
          : readyState.hasMarked
            ? 'Ready — waiting for others'
            : 'I’m Ready'}
      </button>
    </div>
  )
}

function AiSeatManager({
  aiState,
}: {
  aiState?: GameRoomViewProps['aiSeatState']
}) {
  if (!aiState) {
    return (
      <div className="rounded-xl border border-dashed border-slate-800 bg-slate-900/40 p-4 text-xs text-slate-500">
        AI seat controls appear here for the host before the game begins.
      </div>
    )
  }

  const { seats } = aiState
  const registry = aiState.registry
  const registryEntries = registry?.entries ?? []
  const isRegistryLoading = registry?.isLoading ?? false
  const registryError = registry?.error ?? null
  const preferredDefaultName =
    registry?.defaultName ??
    registryEntries.find((entry) => entry.name === 'HeuristicV1')?.name ??
    registryEntries[0]?.name ??
    'HeuristicV1'
  const addDisabled =
    !aiState.canAdd ||
    aiState.isPending ||
    (aiState.registry?.isLoading ?? false)

  return (
    <div className="rounded-xl border border-indigo-500/40 bg-indigo-500/10 p-4 text-sm text-indigo-100">
      <header className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <div>
          <h3 className="text-sm font-semibold text-indigo-200">AI Seats</h3>
          <p className="text-xs text-indigo-100/70">
            Use bots to fill empty seats before the game starts.
          </p>
        </div>
        <span className="rounded-full border border-indigo-400/40 bg-indigo-400/20 px-3 py-1 text-[11px] font-semibold uppercase tracking-wide text-indigo-100">
          {aiState.aiSeats} bots · {aiState.totalSeats - aiState.availableSeats}
          /{aiState.totalSeats} seats filled
        </span>
      </header>

      <div className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center gap-2">
          <button
            type="button"
            onClick={() =>
              aiState.onAdd({ registryName: preferredDefaultName })
            }
            disabled={addDisabled}
            className="rounded-md bg-indigo-400 px-3 py-2 text-sm font-semibold text-slate-900 transition hover:bg-indigo-300 disabled:cursor-not-allowed disabled:bg-indigo-500/40 disabled:text-slate-600"
          >
            {aiState.isPending ? 'Working…' : 'Add AI'}
          </button>
          <span className="text-[11px] text-indigo-100/70">
            Defaults to&nbsp;
            <span className="font-semibold text-indigo-50">
              {preferredDefaultName}
            </span>
            {isRegistryLoading ? ' (loading registry…)' : ''}
          </span>
        </div>

        {registryError ? (
          <div className="rounded-md border border-red-400/40 bg-red-500/10 px-3 py-2 text-xs text-red-200">
            {registryError}
          </div>
        ) : null}

        <ul className="mt-2 space-y-2 text-xs">
          {seats.map((seat, index) => (
            <li
              key={seat.userId ?? `${seat.seat}-${index}`}
              className="rounded-lg border border-indigo-500/20 bg-slate-900/40 px-3 py-3"
            >
              <div className="flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between">
                <div className="flex flex-col">
                  <span className="font-semibold text-indigo-100">
                    Seat {seat.seat + 1}
                  </span>
                  <span className="text-[11px] uppercase tracking-wide text-indigo-200/70">
                    {seat.isOccupied
                      ? [
                          seat.isAi ? 'AI-controlled' : 'Human player',
                          seat.isReady ? 'Ready' : 'Not ready',
                        ].join(' • ')
                      : 'Open seat'}
                  </span>
                  {seat.isAi && seat.aiProfile ? (
                    <span className="text-[11px] text-indigo-200/60">
                      Profile:{' '}
                      <span className="font-medium text-indigo-100">
                        {seat.aiProfile.name}
                      </span>{' '}
                      · v{seat.aiProfile.version}
                    </span>
                  ) : null}
                </div>
                <div className="mt-2 flex items-center gap-2 sm:mt-0">
                  {seat.isAi ? (
                    <>
                      <label
                        htmlFor={`ai-seat-${seat.seat}`}
                        className="sr-only"
                      >
                        Select AI profile for seat {seat.seat + 1}
                      </label>
                      <select
                        id={`ai-seat-${seat.seat}`}
                        className="rounded-md border border-indigo-500/40 bg-slate-900/70 px-2 py-1 text-xs text-indigo-100 focus:border-indigo-300 focus:outline-none focus:ring-2 focus:ring-indigo-400 disabled:cursor-not-allowed disabled:text-indigo-300/60"
                        disabled={
                          aiState.isPending ||
                          isRegistryLoading ||
                          registryEntries.length === 0 ||
                          !aiState.onUpdateSeat
                        }
                        value={
                          seat.aiProfile
                            ? `${seat.aiProfile.name}::${seat.aiProfile.version}`
                            : ''
                        }
                        onChange={(event) => {
                          const value = event.target.value
                          if (!value || !aiState.onUpdateSeat) {
                            return
                          }
                          const [registryName, registryVersion] =
                            value.split('::')
                          aiState.onUpdateSeat(seat.seat, {
                            registryName,
                            registryVersion,
                          })
                        }}
                      >
                        {registryEntries.length === 0 ? (
                          <option value="">
                            {isRegistryLoading
                              ? 'Loading profiles…'
                              : 'No profiles available'}
                          </option>
                        ) : (
                          registryEntries.map((entry) => {
                            const key = `${entry.name}::${entry.version}`
                            return (
                              <option key={key} value={key}>
                                {entry.name} · v{entry.version}
                              </option>
                            )
                          })
                        )}
                      </select>
                      <button
                        type="button"
                        onClick={() => {
                          aiState.onRemoveSeat?.(seat.seat)
                        }}
                        disabled={aiState.isPending}
                        className="inline-flex h-8 w-8 items-center justify-center rounded-md border border-indigo-500/40 text-indigo-100 transition hover:bg-indigo-500/20 disabled:cursor-not-allowed disabled:text-indigo-300/60"
                      >
                        <span className="sr-only">
                          Remove AI from seat {seat.seat + 1}
                        </span>
                        <svg
                          aria-hidden="true"
                          className="h-4 w-4"
                          viewBox="0 0 24 24"
                          fill="none"
                          stroke="currentColor"
                          strokeWidth={1.5}
                          strokeLinecap="round"
                          strokeLinejoin="round"
                        >
                          <path d="M4 7h16" />
                          <path d="M9 7V4h6v3" />
                          <path d="M10 11v6" />
                          <path d="M14 11v6" />
                          <path d="M6 7v12a1 1 0 0 0 1 1h10a1 1 0 0 0 1-1V7" />
                        </svg>
                      </button>
                    </>
                  ) : (
                    <span className="rounded-md border border-indigo-500/20 bg-indigo-500/10 px-2 py-1 text-[11px] text-indigo-100/80">
                      {seat.isOccupied ? 'Human player' : 'Awaiting player'}
                    </span>
                  )}
                </div>
              </div>
            </li>
          ))}
        </ul>
      </div>
    </div>
  )
}

function PhaseFact({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-xs uppercase tracking-wide text-slate-500">{label}</p>
      <p className="text-sm font-medium text-slate-200">{value}</p>
    </div>
  )
}

function getOrientation(
  viewerSeat: Seat,
  seat: Seat
): SeatSummary['orientation'] {
  const relative = (seat - viewerSeat + 4) % 4
  if (relative === 0) return 'bottom'
  if (relative === 1) return 'left'
  if (relative === 2) return 'top'
  return 'right'
}

function getPhaseLabel(phase: PhaseSnapshot): string {
  switch (phase.phase) {
    case 'Init':
      return 'Initializing'
    case 'Bidding':
      return 'Bidding Round'
    case 'TrumpSelect':
      return 'Select Trump'
    case 'Trick':
      return 'Trick Play'
    case 'Scoring':
      return 'Round Scoring'
    case 'Complete':
      return 'Round Complete'
    case 'GameOver':
      return 'Game Over'
    default:
      return 'Unknown Phase'
  }
}

function getRound(phase: PhaseSnapshot): RoundPublic | null {
  switch (phase.phase) {
    case 'Bidding':
    case 'TrumpSelect':
    case 'Trick':
    case 'Scoring':
    case 'Complete':
      return phase.data.round
    default:
      return null
  }
}

function getActiveSeat(phase: PhaseSnapshot): Seat | null {
  switch (phase.phase) {
    case 'Bidding':
    case 'TrumpSelect':
    case 'Trick':
      return phase.data.to_act
    default:
      return null
  }
}

function getCurrentTrickMap(phase: PhaseSnapshot): Map<Seat, Card> {
  if (phase.phase !== 'Trick') {
    return new Map()
  }
  return new Map(phase.data.current_trick)
}

function getBidForSeat(
  phase: PhaseSnapshot,
  seat: Seat
): number | null | undefined {
  if (phase.phase === 'Bidding') {
    return phase.data.bids[seat]
  }
  return undefined
}

function formatTrump(trump: RoundPublic['trump']): string {
  if (!trump) {
    return 'Undeclared'
  }

  switch (trump) {
    case 'CLUBS':
      return 'Clubs'
    case 'DIAMONDS':
      return 'Diamonds'
    case 'HEARTS':
      return 'Hearts'
    case 'SPADES':
      return 'Spades'
    case 'NO_TRUMP':
      return 'No Trump'
    default:
      return trump
  }
}
