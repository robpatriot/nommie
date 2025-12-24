'use client'

import {
  startTransition,
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from 'react'
import { useTranslations } from 'next-intl'
import type { Card, GameSnapshot, Seat } from '@/lib/game-room/types'
import { getPlayerDisplayName } from '@/utils/player-names'
import {
  buildSeatSummaries,
  calculateCardScale,
  getActiveSeat,
  getCurrentTrickMap,
  getRound,
  ORIENTATION_ORDER_MOBILE,
} from './game-room/utils'
import { SeatCard } from './game-room/SeatCard'
import { TrickArea } from './game-room/TrickArea'
import { PlayerHand } from './game-room/PlayerHand'
import { PlayerActions } from './game-room/PlayerActions'
import { ScoreSidebar } from './game-room/ScoreSidebar'
import { ScoreHistoryDialog } from './game-room/ScoreHistoryDialog'
import { AiSeatManager } from './game-room/AiSeatManager'
import { SyncButton } from './game-room/SyncButton'
import { useMediaQuery } from '@/hooks/useMediaQuery'
import { cn } from '@/lib/cn'
import { ReadyPanel } from './game-room/ReadyPanel'
import { SetupSeatList } from './game-room/SetupSeatList'
import { PageHero } from '@/components/PageHero'
import { PageContainer } from '@/components/PageContainer'
import { StatCard } from '@/components/StatCard'
import { useGameHistory } from '@/hooks/queries/useGames'
import { mapGameHistory } from '@/lib/game-room/history-mapping'
import type { GameHistorySummary } from '@/lib/game-room/types'
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
  viewerSeat?: Seat | null
  viewerHand?: Card[]
  onRefresh?: () => void
  isRefreshing?: boolean
  error?: GameRoomError | null
  readyState?: ReadyState
  biddingState?: BiddingState
  trumpState?: TrumpState
  playState?: PlayState
  aiSeatState?: AiSeatState
  status?: {
    lastSyncedAt?: string
  }
  requireCardConfirmation?: boolean
  onCopyInvite?: () => void
}

export function GameRoomView(props: GameRoomViewProps) {
  const t = useTranslations('game.gameRoom')
  const {
    snapshot,
    playerNames,
    viewerSeat,
    viewerHand = [],
    gameId,
    onRefresh,
    isRefreshing = false,
    error,
    readyState,
    biddingState,
    trumpState,
    playState,
    aiSeatState,
    requireCardConfirmation = true,
    onCopyInvite,
  } = props
  const phase = snapshot.phase
  const round = getRound(phase)
  const isPreGame = phase.phase === 'Init'
  const activeSeat = getActiveSeat(phase)

  // Handle viewerSeat explicitly - don't default to 0 to avoid masking missing data.
  // If viewerSeat is undefined/null, no seat will be marked as "viewer".
  const effectiveViewerSeat: Seat | null = viewerSeat ?? null

  const tYou = t('you')
  const tStatusWaiting = t('status.waiting')
  const seatDisplayName = useCallback(
    (seat: Seat) =>
      getPlayerDisplayName(seat, effectiveViewerSeat, playerNames, tYou),
    [playerNames, effectiveViewerSeat, tYou]
  )
  const activeName =
    typeof activeSeat === 'number'
      ? seatDisplayName(activeSeat)
      : tStatusWaiting

  const setupSeatEntries = useMemo(
    () =>
      snapshot.game.seating
        .map((assignment, index) => {
          const seatIndex =
            typeof assignment.seat === 'number' &&
            !Number.isNaN(assignment.seat)
              ? (assignment.seat as Seat)
              : (index as Seat)

          return {
            seat: seatIndex,
            seatNumber: seatIndex + 1,
            name: seatDisplayName(seatIndex),
            isAi: Boolean(assignment.is_ai),
            isReady: Boolean(assignment.is_ready),
            // Treat both humans and AI as occupying a seat for setup stats
            isOccupied:
              Boolean(assignment.user_id) || Boolean(assignment.is_ai),
            isViewer: effectiveViewerSeat === seatIndex,
          }
        })
        .sort((a, b) => a.seat - b.seat),
    [snapshot.game.seating, seatDisplayName, effectiveViewerSeat]
  )

  const totalSeatCount = setupSeatEntries.length
  const { filledSeatCount, aiSeatCount, readySeatCount } = useMemo(
    () =>
      setupSeatEntries.reduce(
        (acc, seat) => ({
          filledSeatCount: acc.filledSeatCount + (seat.isOccupied ? 1 : 0),
          aiSeatCount: acc.aiSeatCount + (seat.isAi ? 1 : 0),
          readySeatCount: acc.readySeatCount + (seat.isReady ? 1 : 0),
        }),
        { filledSeatCount: 0, aiSeatCount: 0, readySeatCount: 0 }
      ),
    [setupSeatEntries]
  )

  const trickMap = getCurrentTrickMap(phase)
  const historicalStats =
    phase.phase === 'Bidding' && phase.data.previous_round
      ? {
          bids: phase.data.previous_round.bids,
          tricksWon: phase.data.previous_round.tricks_won,
        }
      : undefined

  const seatSummaries = buildSeatSummaries({
    playerNames,
    viewerSeat: effectiveViewerSeat ?? 0, // Use 0 as fallback for orientation calculation only
    phase,
    scores: snapshot.game.scores_total,
    trickMap,
    round,
    activeSeat,
    actualViewerSeat: effectiveViewerSeat, // Pass actual viewer seat separately for isViewer check
    historicalStats,
  })
  const mobileSeatSummaries = useMemo(
    () =>
      seatSummaries
        .slice()
        .sort(
          (a, b) =>
            ORIENTATION_ORDER_MOBILE.indexOf(a.orientation) -
            ORIENTATION_ORDER_MOBILE.indexOf(b.orientation)
        ),
    [seatSummaries]
  )

  const [selectedCard, setSelectedCard] = useState<Card | null>(null)
  const [isHistoryOpen, setIsHistoryOpen] = useState(false)
  const showCardWrapper = useMediaQuery('(min-width: 640px)')

  // Use query hook directly
  const {
    data: rawHistory,
    isLoading: isHistoryLoading,
    error: historyQueryError,
    refetch: refetchHistory,
  } = useGameHistory(gameId)

  // Map the raw API response to the expected format
  const history: GameHistorySummary | null = useMemo(() => {
    if (!rawHistory) {
      return null
    }
    return mapGameHistory(rawHistory)
  }, [rawHistory])

  // Convert query error to string
  const historyError = useMemo<string | null>(() => {
    if (!historyQueryError) {
      return null
    }
    return historyQueryError instanceof Error
      ? historyQueryError.message
      : t('history.error.loadFailed')
  }, [historyQueryError, t])

  useEffect(() => {
    if (!selectedCard) {
      return
    }

    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target as Element | null
      if (!target) {
        return
      }

      if (target.closest('[data-selected-card-exempt]')) {
        return
      }

      setSelectedCard(null)
    }

    document.addEventListener('pointerdown', handlePointerDown)
    return () => document.removeEventListener('pointerdown', handlePointerDown)
  }, [selectedCard])

  // Reset and validate selectedCard when phase or playState changes
  // Use startTransition to mark as non-urgent update to avoid cascading renders
  useEffect(() => {
    const currentPhase = phase.phase

    startTransition(() => {
      setSelectedCard((current) => {
        // Reset when not in Trick phase or playState unavailable
        if (currentPhase !== 'Trick' || !playState) {
          return null
        }

        // Reset if current card is not playable
        if (current && !playState.playable.includes(current)) {
          return null
        }

        return current
      })
    })
  }, [phase, playState])

  useEffect(() => {
    if (!isHistoryOpen) {
      return
    }
    void refetchHistory()
  }, [isHistoryOpen, refetchHistory])

  // Get last trick from backend snapshot
  // - In Trick phase: last trick from current round
  // - In Bidding/TrumpSelect: final trick from previous round
  const lastTrick =
    phase.phase === 'Trick' ||
    phase.phase === 'Bidding' ||
    phase.phase === 'TrumpSelect'
      ? phase.data.last_trick
      : null

  const showPreviousRoundPosition =
    phase.phase === 'Bidding' &&
    trickMap.size === 0 &&
    Boolean(lastTrick && lastTrick.length > 0)

  const handlePlayCard = useCallback(
    async (card: Card) => {
      if (!playState) {
        return
      }
      await playState.onPlay(card)
      setSelectedCard(null)
    },
    [playState, setSelectedCard]
  )

  const handleOpenHistory = useCallback(() => {
    setIsHistoryOpen(true)
  }, [])

  const handleCloseHistory = useCallback(() => {
    setIsHistoryOpen(false)
  }, [])

  // Track PlayerHand viewport width for card scaling
  const playerHandViewportRef = useRef<HTMLDivElement>(null)
  const [cardScale, setCardScale] = useState(1)

  useLayoutEffect(() => {
    const viewport = playerHandViewportRef.current
    if (!viewport) {
      return
    }

    const updateScale = () => {
      const width = viewport.clientWidth
      const scale = calculateCardScale(width, viewerHand.length)
      setCardScale(scale)
    }

    updateScale()

    const resizeObserver = new ResizeObserver(() => {
      updateScale()
    })
    resizeObserver.observe(viewport)

    return () => {
      resizeObserver.disconnect()
    }
  }, [viewerHand.length])

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
                    {t('setup.kicker', { gameId })}
                  </p>
                  <h2 className="text-3xl font-semibold text-foreground sm:text-4xl">
                    {t('setup.title')}
                  </h2>
                  <p className="text-sm text-muted sm:text-base">
                    {t('setup.description')}
                  </p>
                </div>
                <ReadyPanel readyState={readyState} />
              </div>
            }
            aside={
              <>
                <div className="grid gap-3 sm:grid-cols-3">
                  <StatCard
                    label={t('setup.stats.totalPlayers.label')}
                    value={`${filledSeatCount}/${totalSeatCount}`}
                    description={t('setup.stats.totalPlayers.description')}
                  />
                  <StatCard
                    label={t('setup.stats.aiPlayers.label')}
                    value={aiSeatCount}
                    description={t('setup.stats.aiPlayers.description')}
                  />
                  <StatCard
                    label={t('setup.stats.readyPlayers.label')}
                    value={`${readySeatCount}/${totalSeatCount}`}
                    description={t('setup.stats.readyPlayers.description')}
                  />
                </div>
                <div className="rounded-2xl border border-border/60 bg-surface/70 p-4">
                  <div className="flex flex-wrap items-center justify-between gap-2 text-[11px] font-semibold uppercase tracking-[0.35em] text-subtle">
                    <span>{t('setup.quickActions.title')}</span>
                    <span className="text-[10px] font-normal tracking-[0.2em] text-muted">
                      {t('setup.quickActions.subtitle')}
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
                            ? t('actions.refreshingAria')
                            : t('actions.refreshAria')
                        }
                      >
                        <div className="space-y-1">
                          <p className="text-[11px] font-semibold uppercase tracking-[0.35em] text-subtle">
                            {t('setup.quickActions.manualSync')}
                          </p>
                          <p className="text-base font-semibold text-foreground">
                            {isRefreshing
                              ? t('actions.refreshingLabel')
                              : t('actions.refreshLabel')}
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
                      onClick={onCopyInvite}
                      className="group flex h-full items-center justify-between rounded-2xl border border-border/60 bg-background/40 px-4 py-3 text-left transition hover:border-primary/50 hover:bg-primary/5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40"
                      aria-label={t('setup.quickActions.copyInviteAria')}
                    >
                      <div className="space-y-1">
                        <p className="text-[11px] font-semibold uppercase tracking-[0.35em] text-subtle">
                          {t('setup.quickActions.shareLink')}
                        </p>
                        <p className="text-base font-semibold text-foreground">
                          {t('setup.quickActions.copyInvite')}
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

  return (
    <div className="flex flex-col text-foreground">
      <PageContainer className="pb-16">
        <div className="grid gap-6 grid-cols-1 lg:grid-cols-[minmax(0,1fr)_320px]">
          <section
            className={cn(
              'relative flex flex-col gap-6',
              showCardWrapper &&
                'card-wrapper rounded-[40px] border border-white/10 px-6 pt-[44px] pb-6 shadow-elevated'
            )}
          >
            {onRefresh ? (
              <div className="pointer-events-auto absolute right-6 top-6 z-10 hidden sm:block">
                <SyncButton onRefresh={onRefresh} isRefreshing={isRefreshing} />
              </div>
            ) : null}
            {showPreviousRoundPosition ? (
              <div className="text-left text-xs font-semibold uppercase tracking-[0.35em] text-subtle">
                {t('trickArea.lastRoundFinalPosition')}
              </div>
            ) : null}
            <div
              className="hidden gap-3 sm:grid"
              style={{
                gridTemplateColumns:
                  'minmax(0,1fr) minmax(0,2.2fr) minmax(0,1fr)',
                gridTemplateRows: 'auto 1fr auto',
              }}
            >
              {seatSummaries.map((summary) => (
                <SeatCard key={summary.seat} summary={summary} />
              ))}
              <TrickArea
                trickMap={trickMap}
                getSeatName={seatDisplayName}
                round={round}
                phase={phase}
                viewerSeat={effectiveViewerSeat ?? 0}
                lastTrick={lastTrick}
                showPreviousRoundPosition={showPreviousRoundPosition}
                className="col-start-2 row-start-2 w-full"
                onRefresh={onRefresh}
                isRefreshing={isRefreshing}
                cardScale={cardScale}
              />
            </div>
            <div className="flex flex-col gap-3 sm:hidden">
              <div className="grid grid-cols-2 gap-3">
                {mobileSeatSummaries.map((summary) => (
                  <SeatCard
                    key={`mobile-${summary.seat}`}
                    summary={summary}
                    variant="list"
                    className="w-full"
                  />
                ))}
              </div>
              <TrickArea
                trickMap={trickMap}
                getSeatName={seatDisplayName}
                round={round}
                phase={phase}
                viewerSeat={effectiveViewerSeat ?? 0}
                lastTrick={lastTrick}
                showPreviousRoundPosition={showPreviousRoundPosition}
                onRefresh={onRefresh}
                isRefreshing={isRefreshing}
                cardScale={cardScale}
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
              onPlayCard={handlePlayCard}
              requireCardConfirmation={requireCardConfirmation}
              layoutVariant="scaled"
              viewportRef={playerHandViewportRef}
            />
          </section>

          <aside className="flex flex-col gap-4 lg:sticky lg:top-6">
            <PlayerActions
              phase={phase}
              viewerSeat={effectiveViewerSeat ?? 0}
              playerNames={playerNames}
              bidding={biddingState}
              trump={trumpState}
              lastTrick={lastTrick}
              seatDisplayName={seatDisplayName}
            />
            <ScoreSidebar
              gameId={gameId}
              phase={phase}
              activeName={activeName}
              playerNames={playerNames}
              scores={snapshot.game.scores_total}
              round={round}
              roundNo={snapshot.game.round_no}
              dealer={snapshot.game.dealer}
              seatDisplayName={seatDisplayName}
              error={error}
              onShowHistory={handleOpenHistory}
              isHistoryLoading={isHistoryLoading}
            />
          </aside>
        </div>
      </PageContainer>
      <ScoreHistoryDialog
        isOpen={isHistoryOpen}
        onClose={handleCloseHistory}
        rounds={history?.rounds ?? []}
        playerNames={playerNames}
        seatDisplayName={seatDisplayName}
        isLoading={isHistoryLoading}
        error={historyError}
      />
    </div>
  )
}
