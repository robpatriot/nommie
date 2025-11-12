'use client'

import { useCallback, useEffect, useState } from 'react'
import Link from 'next/link'

import type { Card, GameSnapshot, Seat } from '@/lib/game-room/types'
import { formatTime } from '@/utils/date-formatting'
import {
  buildSeatSummaries,
  formatTrump,
  getActiveSeat,
  getCurrentTrickMap,
  getPhaseLabel,
  getRound,
} from './game-room/utils'
import { PhaseFact } from './game-room/PhaseFact'
import { SeatCard } from './game-room/SeatCard'
import { TrickArea } from './game-room/TrickArea'
import { PlayerHand } from './game-room/PlayerHand'
import { PlayerActions } from './game-room/PlayerActions'
import { ScoreSidebar } from './game-room/ScoreSidebar'
import type {
  AiSeatState,
  BiddingState,
  GameRoomError,
  GameRoomStatus,
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
  status: GameRoomStatus
  onRefresh?: () => void
  isRefreshing?: boolean
  error?: GameRoomError | null
  readyState?: ReadyState
  biddingState?: BiddingState
  trumpState?: TrumpState
  playState?: PlayState
  aiSeatState?: AiSeatState
}

export function GameRoomView(props: GameRoomViewProps) {
  const {
    snapshot,
    playerNames,
    viewerSeat,
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
  const syncLabel = formatTime(status.lastSyncedAt, {
    hour: '2-digit',
    minute: '2-digit',
  })

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

  const [selectedCard, setSelectedCard] = useState<Card | null>(null)

  // Reset selected card when phase changes or card becomes unplayable
  // No cleanup needed: This effect only updates state, which doesn't require cleanup
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
    <div className="flex min-h-screen flex-col bg-background text-foreground">
      <header className="border-b border-border bg-surface/80 backdrop-blur">
        <div className="mx-auto flex w-full max-w-7xl flex-wrap items-center justify-between gap-2 px-4 py-4 sm:px-6 lg:px-10">
          <div className="flex flex-1 flex-col gap-1">
            <span className="text-sm font-medium text-subtle">
              Game #{gameId}
            </span>
            <h1 className="text-2xl font-semibold text-foreground">
              Nommie Table
            </h1>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            {onRefresh ? (
              <button
                type="button"
                onClick={onRefresh}
                className="rounded-md border border-border px-3 py-1.5 text-sm font-medium text-muted transition hover:border-primary/40 hover:text-foreground"
                disabled={isRefreshing}
                aria-label={
                  isRefreshing ? 'Refreshing game state' : 'Refresh game state'
                }
              >
                {isRefreshing ? 'Refreshing…' : 'Refresh'}
              </button>
            ) : null}
            <button
              type="button"
              onClick={() => {
                const url = window.location.href
                void navigator.clipboard.writeText(url)
              }}
              className="rounded-md border border-border px-3 py-1.5 text-sm font-medium text-muted transition hover:border-accent/50 hover:text-foreground"
              aria-label="Copy invite link to clipboard"
            >
              Copy Invite Link
            </button>
            <Link
              href="/lobby"
              className="rounded-md bg-primary px-3 py-1.5 text-sm font-semibold text-primary-foreground transition hover:bg-primary/90"
            >
              Back to Lobby
            </Link>
          </div>
        </div>
      </header>

      <main className="flex flex-1 flex-col gap-6 px-4 py-6 sm:px-6 lg:px-10">
        <section className="flex flex-col gap-4 rounded-xl border border-border bg-surface/70 p-4 shadow-elevated">
          <div className="flex flex-wrap items-center justify-between gap-4">
            <div>
              <p className="text-sm uppercase tracking-wide text-subtle">
                Phase
              </p>
              <div className="text-2xl font-semibold text-foreground">
                {getPhaseLabel(phase)}
              </div>
            </div>
            <div className="flex items-center gap-3 text-sm text-muted">
              <span className="flex items-center gap-2">
                <span
                  className={`inline-flex h-2.5 w-2.5 items-center justify-center rounded-full ${
                    status.isPolling ? 'animate-pulse bg-success' : 'bg-subtle'
                  }`}
                  aria-hidden
                />
                {status.isPolling ? 'Syncing…' : 'Idle'}
              </span>
              <span aria-live="off" className="text-subtle">
                Last synced {syncLabel}
              </span>
            </div>
          </div>
          {error ? (
            <div className="rounded-lg border border-warning/60 bg-warning/10 px-3 py-2 text-sm text-warning-foreground">
              <p>{error.message}</p>
              {error.traceId ? (
                <p className="text-xs text-warning-foreground/80">
                  traceId: {error.traceId}
                </p>
              ) : null}
            </div>
          ) : null}
          {round ? (
            <div className="grid gap-3 text-sm text-muted sm:grid-cols-4">
              <PhaseFact label="Round" value={`#${snapshot.game.round_no}`} />
              <PhaseFact label="Hand Size" value={round.hand_size.toString()} />
              <PhaseFact
                label="Dealer"
                value={seatDisplayName(snapshot.game.dealer)}
              />
              <PhaseFact label="Trump" value={formatTrump(round.trump)} />
            </div>
          ) : null}
          <div className="flex flex-wrap items-center gap-4 text-sm text-muted">
            <span className="rounded-full bg-surface px-3 py-1 font-medium text-foreground">
              Turn: {activeName}
            </span>
            {phase.phase === 'Trick' ? (
              <span className="text-subtle">
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
            />

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
          </div>

          <ScoreSidebar
            playerNames={playerNames}
            scores={snapshot.game.scores_total}
            round={round}
            readyState={readyState}
            aiState={aiSeatState}
            isPreGame={isPreGame}
          />
        </section>
      </main>
    </div>
  )
}
