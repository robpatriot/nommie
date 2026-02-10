import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { ReactNode } from 'react'
import { fireEvent } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

import { GameRoomView } from '@/app/game/[gameId]/_components/game-room-view'
import {
  biddingSnapshotFixture,
  initSnapshotFixture,
  trickSnapshotFixture,
} from '../mocks/game-snapshot'
import { createTestQueryClient, render, screen, waitFor } from '../utils'
import { queryKeys } from '@/lib/queries/query-keys'

vi.mock('next/link', () => ({
  __esModule: true,
  default: ({ children, ...props }: { children: ReactNode; href: string }) => (
    <a {...props}>{children}</a>
  ),
}))

// Mock server actions - hoist so tests can mutate behavior per-case
const { mockGetGameHistoryAction } = vi.hoisted(() => {
  return { mockGetGameHistoryAction: vi.fn() }
})

vi.mock('@/app/actions/game-actions', () => ({
  getGameHistoryAction: (gameId: number) => mockGetGameHistoryAction(gameId),
  refreshGamesListAction: vi.fn(),
}))

describe('GameRoomView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockGetGameHistoryAction.mockResolvedValue({
      kind: 'ok',
      data: { rounds: [] },
    })
  })
  const playerNames: [string, string, string, string] = [
    'Alex',
    'Bailey',
    'Casey',
    'Dakota',
  ]

  it('renders phase summary and seat information', () => {
    render(
      <GameRoomView
        gameId={42}
        snapshot={biddingSnapshotFixture}
        playerNames={playerNames}
        viewerSeat={0}
        viewerHand={['2H', '3C']}
        status={{
          lastSyncedAt: new Date('2025-01-06T15:04:05Z').toISOString(),
        }}
        onRefresh={() => undefined}
      />
    )

    expect(screen.getByText('Bidding Round')).toBeInTheDocument()
    expect(screen.getAllByText('Alex').length).toBeGreaterThan(0)
    expect(screen.getAllByText(/Won/)[0].textContent).toContain('2')
    expect(screen.getAllByText('Bid 2').length).toBeGreaterThan(0)
    // Refresh button is only shown in Init phase, not during active game phases
  })

  it('surfaces errors', () => {
    render(
      <GameRoomView
        gameId={42}
        snapshot={biddingSnapshotFixture}
        playerNames={playerNames}
        viewerSeat={0}
        viewerHand={[]}
        status={{ lastSyncedAt: new Date().toISOString() }}
        error={{ message: 'Sync failed', traceId: 'abc123' }}
      />
    )

    expect(screen.getByText('Sync failed')).toBeInTheDocument()
    expect(screen.getByText(/traceId: abc123/)).toBeInTheDocument()
  })

  it('renders bidding controls and submits bid for the active viewer', async () => {
    const onSubmit = vi.fn().mockResolvedValue(undefined)

    render(
      <GameRoomView
        gameId={42}
        snapshot={biddingSnapshotFixture}
        playerNames={playerNames}
        viewerSeat={1}
        viewerHand={[]}
        status={{ lastSyncedAt: new Date().toISOString() }}
        biddingState={{
          viewerSeat: 1,
          isPending: false,
          onSubmit,
        }}
      />
    )

    const bidInput = screen.getByLabelText('Bid value') as HTMLInputElement
    expect(bidInput.value).toBe('')

    fireEvent.change(bidInput, { target: { value: '4' } })
    expect(bidInput.value).toBe('4')

    // aria-label is dynamic: "Submit bid of ${selectedBid}"
    const submitButton = screen.getByRole('button', {
      name: /Submit bid of 4/i,
    })
    expect(submitButton).toBeEnabled()

    await userEvent.click(submitButton)
    expect(onSubmit).toHaveBeenCalledWith(4)
  })

  it('shows previous round bids and tricks during bidding', () => {
    render(
      <GameRoomView
        gameId={77}
        snapshot={biddingSnapshotFixture}
        playerNames={playerNames}
        viewerSeat={0}
        viewerHand={[]}
        status={{ lastSyncedAt: new Date().toISOString() }}
      />
    )

    expect(screen.getAllByText('Won 2').length).toBeGreaterThan(0)
    expect(screen.getAllByText('Bid 3').length).toBeGreaterThan(0)
    expect(screen.getByText("Last round's final position")).toBeInTheDocument()
  })

  it('opens score history dialog when requested', async () => {
    const historyPayload = {
      rounds: [
        {
          round_no: 1,
          hand_size: 7,
          dealer_seat: 0,
          trump_selector_seat: 2,
          trump: 'HEARTS',
          bids: [3, 1, 4, 0] as [
            number | null,
            number | null,
            number | null,
            number | null,
          ],
          cumulative_scores: [13, 4, 17, 2] as [number, number, number, number],
        },
      ],
    }

    mockGetGameHistoryAction.mockResolvedValue({
      kind: 'ok',
      data: historyPayload,
    })

    const biddingRound2Snapshot = {
      ...biddingSnapshotFixture,
      game: {
        ...biddingSnapshotFixture.game,
        round_no: 2,
        scores_total: [13, 4, 17, 2] as [number, number, number, number],
      },
    }

    render(
      <GameRoomView
        gameId={42}
        snapshot={biddingRound2Snapshot}
        playerNames={playerNames}
        viewerSeat={0}
        viewerHand={[]}
        status={{ lastSyncedAt: new Date().toISOString() }}
      />
    )

    // History is fetched on mount (not on dialog open).
    await waitFor(() => {
      expect(mockGetGameHistoryAction).toHaveBeenCalledTimes(1)
      expect(mockGetGameHistoryAction).toHaveBeenCalledWith(42)
    })

    const historyButton = screen.getByRole('button', {
      name: /show score history/i,
    })
    await userEvent.click(historyButton)

    // Opening the dialog should not trigger a refetch by itself.
    expect(mockGetGameHistoryAction).toHaveBeenCalledTimes(1)

    expect(await screen.findByText('Score sheet')).toBeInTheDocument()
    expect(await screen.findByText('Round 1')).toBeInTheDocument()
    expect(screen.getAllByText('Bid 3').length).toBeGreaterThan(0)
    expect(screen.getAllByText('17').length).toBeGreaterThan(0)
  })

  it('refreshes history after returning in bidding when cache is behind', async () => {
    const gameId = 42
    const queryClient = createTestQueryClient()

    let serverHistory = {
      rounds: [
        {
          round_no: 4,
          hand_size: 3,
          dealer_seat: 0,
          trump_selector_seat: 1,
          trump: 'HEARTS',
          bids: [1, 1, 1, 0] as [
            number | null,
            number | null,
            number | null,
            number | null,
          ],
          cumulative_scores: [10, 10, 10, 10] as [
            number,
            number,
            number,
            number,
          ],
        },
      ],
    }

    mockGetGameHistoryAction.mockImplementation(async () => ({
      kind: 'ok',
      data: serverHistory,
    }))

    const { unmount } = render(
      <GameRoomView
        gameId={gameId}
        snapshot={trickSnapshotFixture}
        playerNames={playerNames}
        viewerSeat={0}
        viewerHand={['2H', '3C']}
        status={{ lastSyncedAt: new Date().toISOString() }}
      />,
      { queryClient }
    )

    await waitFor(() => {
      expect(mockGetGameHistoryAction).toHaveBeenCalledTimes(1)
    })

    unmount()

    // While the user is away, round 5 completes and the game enters bidding for round 6.
    serverHistory = {
      rounds: [
        ...serverHistory.rounds,
        {
          round_no: 5,
          hand_size: 2,
          dealer_seat: 3,
          trump_selector_seat: 2,
          trump: 'SPADES',
          bids: [0, 1, 0, 1] as [
            number | null,
            number | null,
            number | null,
            number | null,
          ],
          cumulative_scores: [12, 21, 14, 20] as [
            number,
            number,
            number,
            number,
          ],
        },
      ],
    }

    const biddingRound6Snapshot = {
      ...biddingSnapshotFixture,
      game: {
        ...biddingSnapshotFixture.game,
        round_no: 6,
        scores_total: [12, 21, 14, 20] as [number, number, number, number],
      },
    }

    render(
      <GameRoomView
        gameId={gameId}
        snapshot={biddingRound6Snapshot}
        playerNames={playerNames}
        viewerSeat={0}
        viewerHand={[]}
        status={{ lastSyncedAt: new Date().toISOString() }}
      />,
      { queryClient }
    )

    await waitFor(() => {
      expect(mockGetGameHistoryAction).toHaveBeenCalledTimes(2)
    })

    await waitFor(() => {
      const data = queryClient.getQueryData(queryKeys.games.history(gameId)) as
        | typeof serverHistory
        | undefined
      expect(data?.rounds.some((r) => r.round_no === 5)).toBe(true)
    })
  })

  it('refreshes history on game over when cache is behind', async () => {
    const gameId = 77
    const queryClient = createTestQueryClient()

    let serverHistory = {
      rounds: [
        {
          round_no: 7,
          hand_size: 1,
          dealer_seat: 0,
          trump_selector_seat: 0,
          trump: 'NO_TRUMPS',
          bids: [0, 0, 0, 0] as [
            number | null,
            number | null,
            number | null,
            number | null,
          ],
          cumulative_scores: [30, 28, 27, 31] as [
            number,
            number,
            number,
            number,
          ],
        },
      ],
    }

    mockGetGameHistoryAction.mockImplementation(async () => ({
      kind: 'ok',
      data: serverHistory,
    }))

    const baseSnapshot = {
      ...biddingSnapshotFixture,
      game: { ...biddingSnapshotFixture.game, round_no: 8 },
    }

    const { unmount } = render(
      <GameRoomView
        gameId={gameId}
        snapshot={baseSnapshot}
        playerNames={playerNames}
        viewerSeat={0}
        viewerHand={[]}
        status={{ lastSyncedAt: new Date().toISOString() }}
      />,
      { queryClient }
    )

    await waitFor(() => {
      expect(mockGetGameHistoryAction).toHaveBeenCalledTimes(1)
    })

    unmount()

    // Final round is now recorded in history, and user returns on GameOver.
    serverHistory = {
      rounds: [
        ...serverHistory.rounds,
        {
          round_no: 8,
          hand_size: 1,
          dealer_seat: 1,
          trump_selector_seat: 2,
          trump: 'DIAMONDS',
          bids: [0, 0, 1, 0] as [
            number | null,
            number | null,
            number | null,
            number | null,
          ],
          cumulative_scores: [40, 38, 39, 41] as [
            number,
            number,
            number,
            number,
          ],
        },
      ],
    }

    const gameOverSnapshot = {
      ...baseSnapshot,
      game: {
        ...baseSnapshot.game,
        scores_total: [40, 38, 39, 41] as [number, number, number, number],
      },
      phase: { phase: 'GameOver' as const },
    }

    render(
      <GameRoomView
        gameId={gameId}
        snapshot={gameOverSnapshot}
        playerNames={playerNames}
        viewerSeat={0}
        viewerHand={[]}
        status={{ lastSyncedAt: new Date().toISOString() }}
      />,
      { queryClient }
    )

    await waitFor(() => {
      expect(mockGetGameHistoryAction).toHaveBeenCalledTimes(2)
    })

    await waitFor(() => {
      const data = queryClient.getQueryData(queryKeys.games.history(gameId)) as
        | typeof serverHistory
        | undefined
      expect(data?.rounds.some((r) => r.round_no === 8)).toBe(true)
    })
  })

  it('enforces legal card gating and triggers play submission', async () => {
    const onPlay = vi.fn().mockResolvedValue(undefined)

    // Extract playable cards from trick snapshot (type-safe)
    const playableCards =
      trickSnapshotFixture.phase.phase === 'Trick'
        ? trickSnapshotFixture.phase.data.playable
        : []

    render(
      <GameRoomView
        gameId={99}
        snapshot={trickSnapshotFixture}
        playerNames={playerNames}
        viewerSeat={0}
        viewerHand={['2H', 'KD', 'QC', 'AS']}
        status={{ lastSyncedAt: new Date().toISOString() }}
        playState={{
          viewerSeat: 0,
          playable: playableCards,
          isPending: false,
          onPlay,
        }}
      />
    )

    expect(screen.getByText('Legal cards:')).toBeInTheDocument()
    // The implementation shows the suit letter when player has cards matching lead suit
    // Lead card is 'AS' (Spades), and viewer has 'AS' in hand, so it displays 'S'
    expect(screen.getByText('S')).toBeInTheDocument()

    // aria-label format: "${card}, ${isSelected ? 'selected' : 'playable'}" or "${card}, not playable"
    const legalCardButton = screen.getByRole('button', { name: /^2H,/i })
    expect(legalCardButton).toBeEnabled()

    const illegalCardButton = screen.getByRole('button', { name: /^AS,/i })
    expect(illegalCardButton).toBeDisabled()

    await userEvent.click(legalCardButton)
    const playButton = screen.getByRole('button', {
      name: /Play selected card/i,
    })
    await userEvent.click(playButton)

    expect(onPlay).toHaveBeenCalledWith('2H')
  })

  it('keeps bids visible during trick phase', () => {
    render(
      <GameRoomView
        gameId={55}
        snapshot={trickSnapshotFixture}
        playerNames={playerNames}
        viewerSeat={0}
        viewerHand={['2H', 'KD']}
        status={{ lastSyncedAt: new Date().toISOString() }}
      />
    )

    expect(screen.getAllByText('Bid 5').length).toBeGreaterThan(0)
    expect(screen.getAllByText('Bid 3').length).toBeGreaterThan(0)
  })

  it('plays a card immediately when confirmation is disabled', async () => {
    const onPlay = vi.fn().mockResolvedValue(undefined)

    const playableCards =
      trickSnapshotFixture.phase.phase === 'Trick'
        ? trickSnapshotFixture.phase.data.playable
        : []

    render(
      <GameRoomView
        gameId={99}
        snapshot={trickSnapshotFixture}
        playerNames={playerNames}
        viewerSeat={0}
        viewerHand={['2H', 'KD', 'QC', 'AS']}
        status={{ lastSyncedAt: new Date().toISOString() }}
        playState={{
          viewerSeat: 0,
          playable: playableCards,
          isPending: false,
          onPlay,
        }}
        requireCardConfirmation={false}
      />
    )

    expect(
      screen.queryByRole('button', { name: /Play selected card/i })
    ).not.toBeInTheDocument()

    const legalCardButton = screen.getByRole('button', { name: /^2H,/i })
    await userEvent.click(legalCardButton)

    expect(onPlay).toHaveBeenCalledWith('2H')
  })

  it('renders AI management panel for host controls before the game starts', async () => {
    const onAdd = vi.fn()
    const onRemoveSeat = vi.fn()
    const onUpdateSeat = vi.fn()

    render(
      <GameRoomView
        gameId={77}
        snapshot={initSnapshotFixture}
        playerNames={playerNames}
        viewerSeat={0}
        viewerHand={[]}
        status={{ lastSyncedAt: new Date().toISOString() }}
        aiSeatState={{
          totalSeats: 4,
          availableSeats: 1,
          aiSeats: 2,
          isPending: false,
          canAdd: true,
          canRemove: true,
          onAdd,
          onRemoveSeat,
          onUpdateSeat,
          registry: {
            entries: [
              { name: 'Tactician', version: '1.4.0' },
              { name: 'RandomPlayer', version: '1.0.0' },
            ],
            isLoading: false,
            defaultName: 'Tactician',
          },
          seats: [
            {
              seat: 0,
              name: 'Alex',
              userId: 101,
              isOccupied: true,
              isAi: false,
              isReady: true,
              aiProfile: null,
            },
            {
              seat: 1,
              name: 'Bot Bailey',
              userId: 202,
              isOccupied: true,
              isAi: true,
              isReady: true,
              aiProfile: { name: 'Tactician', version: '1.4.0' },
            },
            {
              seat: 2,
              name: 'Bot Casey',
              userId: 303,
              isOccupied: true,
              isAi: true,
              isReady: false,
              aiProfile: { name: 'Tactician', version: '1.4.0' },
            },
            {
              seat: 3,
              name: 'Open',
              userId: null,
              isOccupied: false,
              isAi: false,
              isReady: false,
              aiProfile: null,
            },
          ],
        }}
      />
    )

    expect(screen.getByText('AI Seats')).toBeInTheDocument()
    expect(screen.getByText(/2 bots · 3\/4 seats filled/)).toBeInTheDocument()

    // aria-label format: "Add AI player with profile ${preferredDefaultName}"
    const addButton = screen.getByRole('button', {
      name: /Add AI player with profile/i,
    })
    // Seat 1 (index 1, Bot Bailey) is displayed as "seat 2" in UI (seat + 1)
    // Seat 2 (index 2, Bot Casey) is displayed as "seat 3" in UI (seat + 1)
    // The test updates seat 1 (Bot Bailey), which is displayed as "seat 2"
    const profileSelect = screen.getByLabelText(
      'Select AI profile for seat 2'
    ) as HTMLSelectElement
    // Remove button for seat 1 (index 1, displayed as "seat 2")
    const removeSeatButton = screen.getByRole('button', {
      name: 'Remove AI from seat 2',
    })

    await userEvent.click(addButton)
    await userEvent.selectOptions(profileSelect, 'RandomPlayer::1.0.0')
    await userEvent.click(removeSeatButton)

    expect(onAdd).toHaveBeenCalledWith({ registryName: 'Tactician' })
    expect(onUpdateSeat).toHaveBeenCalledWith(1, {
      registryName: 'RandomPlayer',
      registryVersion: '1.0.0',
    })
    expect(onRemoveSeat).toHaveBeenCalledWith(1)
  })

  it('shows guidance for non-host players before the game starts', () => {
    render(
      <GameRoomView
        gameId={55}
        snapshot={initSnapshotFixture}
        playerNames={playerNames}
        viewerSeat={2}
        viewerHand={[]}
        status={{ lastSyncedAt: new Date().toISOString() }}
      />
    )

    expect(
      screen.getByText(
        /The host is configuring AI players for this game\. Seating updates will appear once the match begins\./
      )
    ).toBeInTheDocument()
  })

  it('hides AI management once the game has started', () => {
    const onAdd = vi.fn()
    const onRemoveSeat = vi.fn()
    const onUpdateSeat = vi.fn()

    render(
      <GameRoomView
        gameId={88}
        snapshot={biddingSnapshotFixture}
        playerNames={playerNames}
        viewerSeat={0}
        viewerHand={[]}
        status={{ lastSyncedAt: new Date().toISOString() }}
        aiSeatState={{
          totalSeats: 4,
          availableSeats: 1,
          aiSeats: 2,
          isPending: false,
          canAdd: true,
          canRemove: true,
          onAdd,
          onRemoveSeat,
          onUpdateSeat,
          registry: {
            entries: [{ name: 'Tactician', version: '1.4.0' }],
            isLoading: false,
            defaultName: 'Tactician',
          },
          seats: [],
        }}
      />
    )

    expect(screen.queryByText('AI Seats')).not.toBeInTheDocument()
    expect(
      screen.queryByText(/The host is configuring AI players/)
    ).not.toBeInTheDocument()
  })
})
