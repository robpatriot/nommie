'use client'

import { useCallback, useEffect, useState } from 'react'
import type { Card, GameSnapshot, Seat } from '@/lib/game-room/types'
import {
  buildSeatSummaries,
  formatTrump,
  getActiveSeat,
  getCurrentTrickMap,
  getPhaseLabel,
  getRound,
} from './game-room/utils'
import type { SeatSummary } from './game-room/utils'
import { PhaseFact } from './game-room/PhaseFact'
import { SeatCard } from './game-room/SeatCard'
import { TrickArea } from './game-room/TrickArea'
import { PlayerHand } from './game-room/PlayerHand'
import { PlayerActions } from './game-room/PlayerActions'
import { ScoreSidebar } from './game-room/ScoreSidebar'
import { AiSeatManager } from './game-room/AiSeatManager'
import { ReadyPanel } from './game-room/ReadyPanel'
import { SetupSeatList } from './game-room/SetupSeatList'
import { PageHero } from '@/components/PageHero'
import { PageContainer } from '@/components/PageContainer'
import { StatCard } from '@/components/StatCard'
import type {
  AiSeatState,
  BiddingState,
  GameRoomError,
  PlayState,
  ReadyState,
  TrumpState,
} from './game-room-view.types'

// Re-export types for use in other components (e.g., game-room-client.tsx)
export type { AiSeatSelection, AiSeatState } from './game-room-view.types'

export interface GameRoomViewProps {
  gameId: number
  snapshot: GameSnapshot
  playerNames: [string, string, string, string]
  viewerSeat?: Seat
  viewerHand?: Card[]
  onRefresh?: () => void
  isRefreshing?: boolean
  isSlowSync?: boolean
  error?: GameRoomError | null
  readyState?: ReadyState
  biddingState?: BiddingState
  trumpState?: TrumpState
  playState?: PlayState
  aiSeatState?: AiSeatState
  status?: {
    lastSyncedAt?: string
    isPolling?: boolean
  }
}

export function GameRoomView(props: GameRoomViewProps) {
  const {
    snapshot,
    playerNames,
    viewerSeat,
    viewerHand = [],
    gameId,
    onRefresh,
    isRefreshing = false,
    isSlowSync = false,
    error,
    readyState,
    biddingState,
    trumpState,
    playState,
    aiSeatState,
    status,
  } = props
  const phase = snapshot.phase
  const round = getRound(phase)
  const isPreGame = phase.phase === 'Init'
  const activeSeat = getActiveSeat(phase)

  // Handle viewerSeat explicitly - don't default to 0 to avoid masking missing data.
  // If viewerSeat is undefined/null, no seat will be marked as "viewer".
  const effectiveViewerSeat: Seat | null = viewerSeat ?? null

  const seatDisplayName = useCallback(
    (seat: Seat) =>
      effectiveViewerSeat !== null && seat === effectiveViewerSeat
        ? 'You'
        : playerNames[seat],
    [playerNames, effectiveViewerSeat]
  )
  const activeName =
    typeof activeSeat === 'number' ? seatDisplayName(activeSeat) : 'Waiting'

  const setupSeatEntries = snapshot.game.seating
    .map((assignment, index) => {
      const seatIndex =
        typeof assignment.seat === 'number' && !Number.isNaN(assignment.seat)
          ? (assignment.seat as Seat)
          : (index as Seat)

      return {
        seat: seatIndex,
        seatNumber: seatIndex + 1,
        name: seatDisplayName(seatIndex),
        isAi: Boolean(assignment.is_ai),
        isReady: Boolean(assignment.is_ready),
        // Treat both humans and AI as occupying a seat for setup stats
        isOccupied: Boolean(assignment.user_id) || Boolean(assignment.is_ai),
        isViewer: effectiveViewerSeat === seatIndex,
      }
    })
    .sort((a, b) => a.seat - b.seat)

  const totalSeatCount = setupSeatEntries.length
  const filledSeatCount = setupSeatEntries.filter(
    (seat) => seat.isOccupied
  ).length
  const aiSeatCount = setupSeatEntries.filter((seat) => seat.isAi).length
  const readySeatCount = setupSeatEntries.filter((seat) => seat.isReady).length

  const trickMap = getCurrentTrickMap(phase)
  const seatSummaries = buildSeatSummaries({
    playerNames,
    viewerSeat: effectiveViewerSeat ?? 0, // Use 0 as fallback for orientation calculation only
    phase,
    scores: snapshot.game.scores_total,
    trickMap,
    round,
    activeSeat,
    actualViewerSeat: effectiveViewerSeat, // Pass actual viewer seat separately for isViewer check
  })
  const orientationOrder: SeatSummary['orientation'][] = [
    'bottom',
    'right',
    'top',
    'left',
  ]
  const mobileSeatSummaries = seatSummaries
    .slice()
    .sort(
      (a, b) =>
        orientationOrder.indexOf(a.orientation) -
        orientationOrder.indexOf(b.orientation)
    )

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

  if (isPreGame) {
    return (
      <div className="flex flex-col text-foreground">
        <PageContainer>
          <PageHero
            introClassName="lg:flex-[1.35]"
            intro={
              <div className="flex flex-col gap-4">
                <div className="space-y-2">
                  <p className="text-xs font-semibold uppercase tracking-[0.35em] text-subtle">
                    Setup · Game #{gameId}
                  </p>
                  <h2 className="text-3xl font-semibold text-foreground sm:text-4xl">
                    Add players
                  </h2>
                  <p className="text-sm text-muted sm:text-base">
                    Confirm who is seated, drop in AI partners where needed, and
                    ready up once your table is set. The match begins when every
                    seat is marked ready.
                  </p>
                </div>
                <ReadyPanel readyState={readyState} />
              </div>
            }
            aside={
              <>
                <div className="grid gap-3 sm:grid-cols-3">
                  <StatCard
                    label="Total players"
                    value={`${filledSeatCount}/${totalSeatCount}`}
                    description="Human or AI seats assigned"
                  />
                  <StatCard
                    label="AI players"
                    value={aiSeatCount}
                    description="Bots currently seated"
                  />
                  <StatCard
                    label="Ready players"
                    value={`${readySeatCount}/${totalSeatCount}`}
                    description="Marked ready so far"
                  />
                </div>
                <div className="rounded-3xl border border-white/10 bg-surface/70 p-4 shadow-[0_30px_90px_rgba(0,0,0,0.35)]">
                  <div className="flex flex-wrap items-center justify-between gap-2 text-[11px] font-semibold uppercase tracking-[0.35em] text-subtle">
                    <span>Quick actions</span>
                    <span className="text-[10px] font-normal tracking-[0.2em] text-muted">
                      Stay synced & invite friends
                    </span>
                  </div>
                  <div className="mt-4 grid gap-2 sm:grid-cols-2">
                    {onRefresh ? (
                      <button
                        type="button"
                        onClick={onRefresh}
                        className="group flex h-full items-center justify-between rounded-2xl border border-border/60 bg-background/40 px-4 py-3 text-left transition hover:border-primary/50 hover:bg-primary/5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40 disabled:cursor-not-allowed"
                        disabled={isRefreshing}
                        aria-label={
                          isRefreshing
                            ? 'Refreshing game state'
                            : 'Refresh game state'
                        }
                      >
                        <div className="space-y-1">
                          <p className="text-[11px] font-semibold uppercase tracking-[0.35em] text-subtle">
                            Manual sync
                          </p>
                          <p className="text-base font-semibold text-foreground">
                            {isRefreshing
                              ? 'Refreshing…'
                              : 'Refresh game state'}
                          </p>
                        </div>
                        <span className="flex h-11 w-11 items-center justify-center rounded-xl bg-surface/80 text-foreground transition group-hover:bg-primary/10 group-hover:text-primary">
                          <svg
                            aria-hidden="true"
                            className="h-5 w-5"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            strokeWidth={1.8}
                            strokeLinecap="round"
                            strokeLinejoin="round"
                          >
                            <path d="M21 2v6h-6" />
                            <path d="M3 22v-6h6" />
                            <path d="M3.51 9a9 9 0 0 1 14.85-3.36L21 8" />
                            <path d="M20.49 15a9 9 0 0 1-14.85 3.36L3 16" />
                          </svg>
                        </span>
                      </button>
                    ) : null}
                    <button
                      type="button"
                      onClick={() => {
                        const url = window.location.href
                        void navigator.clipboard.writeText(url)
                      }}
                      className="group flex h-full items-center justify-between rounded-2xl border border-border/60 bg-background/40 px-4 py-3 text-left transition hover:border-primary/50 hover:bg-primary/5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40"
                      aria-label="Copy invite link to clipboard"
                    >
                      <div className="space-y-1">
                        <p className="text-[11px] font-semibold uppercase tracking-[0.35em] text-subtle">
                          Share link
                        </p>
                        <p className="text-base font-semibold text-foreground">
                          Copy invite
                        </p>
                      </div>
                      <span className="flex h-11 w-11 items-center justify-center rounded-xl bg-surface/80 text-foreground transition group-hover:bg-primary/10 group-hover:text-primary">
                        <svg
                          aria-hidden="true"
                          className="h-5 w-5"
                          viewBox="0 0 24 24"
                          fill="none"
                          stroke="currentColor"
                          strokeWidth={1.8}
                          strokeLinecap="round"
                          strokeLinejoin="round"
                        >
                          <path d="M18 13v6a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
                          <path d="M15 3h6v6" />
                          <path d="m10 14 11-11" />
                        </svg>
                      </span>
                    </button>
                  </div>
                  {isSlowSync ? (
                    <div className="mt-4 flex items-center gap-2 rounded-lg border border-primary/40 bg-primary/10 px-3 py-2 text-sm text-primary-foreground">
                      <span className="inline-flex h-2 w-2 animate-pulse items-center justify-center rounded-full bg-primary" />
                      <span>Updating game state…</span>
                    </div>
                  ) : null}
                </div>
              </>
            }
          />

          <span className="sr-only">{phase.phase}</span>

          {error ? (
            <div className="mt-6 rounded-2xl border border-warning/60 bg-warning/10 px-4 py-3 text-sm text-warning-foreground">
              <p>{error.message}</p>
              {error.traceId ? (
                <p className="text-xs text-warning-foreground/80">
                  traceId: {error.traceId}
                </p>
              ) : null}
            </div>
          ) : null}

          <section className="flex flex-col gap-6 lg:flex-row lg:items-start">
            <div className="flex flex-1 flex-col gap-6">
              <SetupSeatList seats={setupSeatEntries} />
            </div>
            <div className="lg:flex-[0.8]">
              <AiSeatManager aiState={aiSeatState} />
            </div>
          </section>
        </PageContainer>
      </div>
    )
  }

  const biddingPhase = phase.phase === 'Bidding' ? phase.data : null
  const bidStatus =
    biddingPhase === null
      ? []
      : ([0, 1, 2, 3] as const).map((seatIndex) => ({
          seat: seatIndex,
          name: seatDisplayName(seatIndex as Seat),
          bid: biddingPhase.bids[seatIndex],
          isActive: biddingPhase.to_act === seatIndex,
        }))
  const isSyncing = Boolean(isRefreshing || status?.isPolling)

  return (
    <div className="flex flex-col text-foreground">
      <PageContainer className="pb-16">
        <section className="rounded-[32px] border border-white/10 bg-surface/70 p-6 shadow-[0_45px_120px_rgba(0,0,0,0.35)]">
          <div className="flex flex-wrap items-center justify-between gap-4">
            <div className="space-y-2">
              <p className="text-xs font-semibold uppercase tracking-[0.4em] text-subtle">
                Game #{gameId}
              </p>
              <div className="text-3xl font-semibold">
                {getPhaseLabel(phase)}
              </div>
              <p className="text-xs font-medium uppercase tracking-[0.35em] text-subtle">
                Turn:{' '}
                <span className="text-sm text-foreground">{activeName}</span>
              </p>
            </div>
            <div className="flex flex-wrap items-center gap-3">
              {isSyncing ? (
                <span className="rounded-full border border-primary/60 bg-primary/10 px-4 py-2 text-xs font-semibold uppercase tracking-[0.35em] text-primary-foreground">
                  Syncing…
                </span>
              ) : null}
              {phase.phase === 'Trick' ? (
                <span className="rounded-full bg-surface px-4 py-2 text-xs font-semibold uppercase tracking-[0.35em] text-subtle">
                  Trick {phase.data.trick_no} / {round?.hand_size ?? '?'}
                </span>
              ) : null}
              {onRefresh ? (
                <button
                  type="button"
                  onClick={onRefresh}
                  disabled={isRefreshing}
                  className="rounded-full border border-white/20 px-4 py-2 text-xs font-semibold uppercase tracking-[0.35em] text-foreground transition hover:border-primary/60 hover:text-primary disabled:opacity-60"
                  aria-label="Refresh game state"
                >
                  {isRefreshing ? 'Syncing…' : 'Refresh'}
                </button>
              ) : null}
            </div>
          </div>
          {round ? (
            <div className="mt-4 grid gap-3 text-sm text-muted md:grid-cols-4">
              <PhaseFact label="Round" value={`#${snapshot.game.round_no}`} />
              <PhaseFact label="Hand Size" value={round.hand_size.toString()} />
              <PhaseFact
                label="Dealer"
                value={seatDisplayName(snapshot.game.dealer)}
              />
              <PhaseFact label="Trump" value={formatTrump(round.trump)} />
            </div>
          ) : null}
          {isSlowSync ? (
            <div className="mt-4 flex items-center gap-2 rounded-lg border border-primary/40 bg-primary/10 px-3 py-2 text-sm text-primary-foreground">
              <span className="inline-flex h-2 w-2 animate-pulse items-center justify-center rounded-full bg-primary" />
              <span>Updating game state…</span>
            </div>
          ) : null}
          {error ? (
            <div className="mt-4 rounded-lg border border-warning/60 bg-warning/10 px-3 py-2 text-sm text-warning-foreground">
              <p>{error.message}</p>
              {error.traceId ? (
                <p className="text-xs text-warning-foreground/80">
                  traceId: {error.traceId}
                </p>
              ) : null}
            </div>
          ) : null}
          {bidStatus.length > 0 ? (
            <div className="mt-4 flex flex-wrap gap-2">
              {bidStatus.map(({ seat, name, bid, isActive }) => (
                <div
                  key={seat}
                  className={`flex items-center gap-3 rounded-2xl border px-4 py-2 text-xs font-semibold uppercase tracking-[0.3em] ${
                    isActive
                      ? 'border-success bg-success/15 text-success-contrast'
                      : 'border-white/15 bg-surface text-muted'
                  }`}
                >
                  <span className="text-[11px] tracking-[0.2em] text-subtle">
                    {name}
                  </span>
                  <span className="text-base font-semibold text-foreground">
                    {bid ?? '—'}
                  </span>
                </div>
              ))}
            </div>
          ) : null}
        </section>

        <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_320px]">
          <section className="flex flex-col gap-6 rounded-[40px] border border-white/10 bg-gradient-to-b from-[rgba(var(--felt-highlight),0.95)] via-[rgba(var(--felt-base),0.95)] to-[rgba(var(--felt-shadow),0.98)] p-6 shadow-[0_60px_140px_rgba(0,0,0,0.45)]">
            <div className="hidden min-h-[420px] grid-cols-3 grid-rows-3 gap-4 lg:grid">
              {seatSummaries.map((summary) => (
                <SeatCard key={summary.seat} summary={summary} />
              ))}
              <TrickArea
                trickMap={trickMap}
                getSeatName={seatDisplayName}
                round={round}
                phase={phase}
                viewerSeat={effectiveViewerSeat ?? 0}
                className="col-start-2 row-start-2 h-full w-full"
              />
            </div>
            <div className="flex flex-col gap-3 lg:hidden">
              {mobileSeatSummaries.map((summary) => (
                <SeatCard
                  key={`mobile-${summary.seat}`}
                  summary={summary}
                  variant="list"
                  showBid={false}
                />
              ))}
            </div>
            <div className="lg:hidden">
              <TrickArea
                trickMap={trickMap}
                getSeatName={seatDisplayName}
                round={round}
                phase={phase}
                viewerSeat={effectiveViewerSeat ?? 0}
              />
            </div>
            <PlayerHand
              viewerHand={viewerHand}
              phase={phase}
              playerNames={playerNames}
              viewerSeat={effectiveViewerSeat ?? 0}
              playState={playState}
              selectedCard={selectedCard}
              onSelectCard={setSelectedCard}
              className="bg-black/40"
            />
          </section>

          <aside className="flex flex-col gap-4 xl:sticky xl:top-6">
            <PlayerActions
              phase={phase}
              viewerSeat={effectiveViewerSeat ?? 0}
              playerNames={playerNames}
              bidding={biddingState}
              trump={trumpState}
              play={playState}
              selectedCard={selectedCard}
              onPlayCard={handlePlayCard}
            />
            <ScoreSidebar
              playerNames={playerNames}
              scores={snapshot.game.scores_total}
              round={round}
            />
          </aside>
        </div>
      </PageContainer>
    </div>
  )
}
