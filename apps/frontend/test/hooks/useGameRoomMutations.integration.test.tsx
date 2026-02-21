// ============================================================================
// FILE: apps/frontend/test/hooks/useGameRoomMutations.integration.test.tsx
// Full mutation cycle integration tests validating optimistic updates and
// WebSocket reconciliation for bid, trump, and card play mutations.
// ============================================================================

import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, waitFor, act, createTestQueryClient } from '../utils'
import type { QueryClient } from '@tanstack/react-query'
import {
  useSubmitBid,
  useSelectTrump,
  useSubmitPlay,
} from '@/hooks/mutations/useGameRoomMutations'
import { useGameSync } from '@/hooks/useGameSync'
import { queryKeys } from '@/lib/queries/query-keys'
import { mockGetGameRoomStateAction } from '../../setupGameRoomActionsMock'
import {
  MockWebSocket,
  mockWebSocketInstances,
} from '@/test/setup/mock-websocket'
import { setupFetchMock } from '../setup/game-room-client-mocks'
import type { GameRoomState } from '@/lib/game-room/state'
import {
  selectSnapshot,
  selectVersion,
  selectViewerHand,
} from '@/lib/game-room/state'
import {
  createInitialStateWithVersion,
  waitForWebSocketConnection,
  sendWebSocketSnapshot,
} from '../setup/game-room-client-helpers'
import {
  biddingSnapshotFixture,
  trickSnapshotFixture,
} from '../mocks/game-snapshot'
import { TEST_BACKEND_WS_URL } from '@/test/setup/test-constants'

const mocks = vi.hoisted(() => ({
  submitBidAction: vi.fn(),
  selectTrumpAction: vi.fn(),
  submitPlayAction: vi.fn(),
  getWaitingLongestGameAction: vi.fn(),
}))

vi.mock('@/app/actions/game-room-actions', async (importOriginal) => {
  const actual = (await importOriginal()) as Record<string, unknown>
  return {
    ...actual,
    submitBidAction: mocks.submitBidAction,
    selectTrumpAction: mocks.selectTrumpAction,
    submitPlayAction: mocks.submitPlayAction,
  }
})

vi.mock('@/app/actions/game-actions', async (importOriginal) => {
  const actual = (await importOriginal()) as Record<string, unknown>
  return {
    ...actual,
    getWaitingLongestGameAction: mocks.getWaitingLongestGameAction,
  }
})

// Track original fetch
const originalFetch = globalThis.fetch

// Mock environment variables
vi.stubGlobal('WebSocket', MockWebSocket)

// Mock WebSocket config validation
vi.mock('@/lib/config/env-validation', () => ({
  resolveWebSocketUrl: () => TEST_BACKEND_WS_URL,
  validateWebSocketConfig: () => {},
}))

// Mock error logger to avoid console noise
vi.mock('@/lib/logging/error-logger', () => ({
  logError: vi.fn(),
}))

describe('Full Mutation Cycle Integration Tests', () => {
  let queryClient: QueryClient

  beforeEach(() => {
    queryClient = createTestQueryClient()

    mockWebSocketInstances.length = 0
    vi.clearAllMocks()
    vi.useRealTimers()

    // Mock fetch for /api/ws-token endpoint
    setupFetchMock(originalFetch)

    // Ensure WebSocket is mocked
    vi.stubGlobal('WebSocket', MockWebSocket)

    mocks.getWaitingLongestGameAction.mockResolvedValue({
      kind: 'ok',
      data: [],
    })

    mockGetGameRoomStateAction.mockResolvedValue({
      kind: 'ok',
      data: createInitialStateWithVersion(1, 1),
    })
  })

  afterEach(() => {
    mockWebSocketInstances.forEach((ws) => {
      ws.onopen = null
      ws.onmessage = null
      ws.onerror = null
      ws.onclose = null
    })
    mockWebSocketInstances.length = 0
    vi.clearAllMocks()
    vi.useRealTimers()
  })

  describe('Bid Submission Cycle', () => {
    it('completes bid submission with optimistic update and WS reconciliation', async () => {
      const gameId = 1
      const initialState = createInitialStateWithVersion(gameId, 1, {
        viewerSeat: 1,
        viewerHand: ['2H', '3C', '4D'],
      })
      initialState.game = biddingSnapshotFixture

      // Set initial cache
      queryClient.setQueryData(queryKeys.games.state(gameId), initialState)

      // Render useGameSync to establish WS connection
      const { result: _syncResult } = renderHook(
        () => useGameSync({ initialState, gameId }),
        { queryClient }
      )

      // Wait for WS connection
      const ws = await waitForWebSocketConnection()

      // Render mutation hook
      const { result: mutationResult } = renderHook(() => useSubmitBid(), {
        queryClient,
      })

      // Mock successful server action
      mocks.submitBidAction.mockResolvedValue({ kind: 'ok' })

      // 1. Submit bid mutation
      await act(async () => {
        await mutationResult.current.mutateAsync({
          gameId,
          bid: 5,
          version: 1,
        })
      })

      // 2. Verify optimistic update applied immediately
      const optimisticState = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(gameId)
      )
      expect(optimisticState).toBeDefined()
      expect(optimisticState!.source).toBe('optimistic')
      const snapshot = selectSnapshot(optimisticState!)
      if (snapshot.phase.phase === 'Bidding') {
        expect(snapshot.phase.data.bids[1]).toBe(5)
      } else {
        throw new Error('Expected Bidding phase')
      }

      // 3. Simulate server broadcasting authoritative state via WS (version 2)
      const updatedSnapshot = {
        ...biddingSnapshotFixture,
        phase: {
          ...biddingSnapshotFixture.phase,
          data: {
            ...(biddingSnapshotFixture.phase.phase === 'Bidding'
              ? biddingSnapshotFixture.phase.data
              : {}),
            bids: [2, 5, null, null],
            to_act: 2,
          },
        },
      }

      sendWebSocketSnapshot(ws, updatedSnapshot as any, gameId, queryClient, {
        version: 2,
        viewerSeat: 1,
        viewerHand: ['2H', '3C', '4D'],
      })

      // 4. Verify authoritative WS state replaced optimistic state
      await waitFor(() => {
        const finalState = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(gameId)
        )
        expect(finalState).toBeDefined()
        expect(selectVersion(finalState!)).toBe(2)
        expect(finalState!.source).toBe('ws')
        const finalSnapshot = selectSnapshot(finalState!)
        if (finalSnapshot.phase.phase === 'Bidding') {
          expect(finalSnapshot.phase.data.bids[1]).toBe(5)
          expect(finalSnapshot.phase.data.to_act).toBe(2)
        }
      })
    })

    it('handles server state correction when optimistic update differs', async () => {
      const gameId = 1
      const initialState = createInitialStateWithVersion(gameId, 1, {
        viewerSeat: 1,
      })
      initialState.game = biddingSnapshotFixture

      queryClient.setQueryData(queryKeys.games.state(gameId), initialState)

      const { result: _syncResult } = renderHook(
        () => useGameSync({ initialState, gameId }),
        { queryClient }
      )

      const ws = await waitForWebSocketConnection()

      const { result: mutationResult } = renderHook(() => useSubmitBid(), {
        queryClient,
      })

      mocks.submitBidAction.mockResolvedValue({ kind: 'ok' })

      // Submit bid 5 optimistically
      await act(async () => {
        await mutationResult.current.mutateAsync({
          gameId,
          bid: 5,
          version: 1,
        })
      })

      // Verify optimistic state
      const currentState = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(gameId)
      )
      const snapshot = selectSnapshot(currentState!)
      if (snapshot.phase.phase === 'Bidding') {
        expect(snapshot.phase.data.bids[1]).toBe(5)
      }

      // Server broadcasts different bid (6, perhaps bid was adjusted due to min/max)
      const correctedSnapshot = {
        ...biddingSnapshotFixture,
        phase: {
          ...biddingSnapshotFixture.phase,
          data: {
            ...(biddingSnapshotFixture.phase.phase === 'Bidding'
              ? biddingSnapshotFixture.phase.data
              : {}),
            bids: [2, 6, null, null], // Server corrected to 6
            to_act: 2,
          },
        },
      }

      sendWebSocketSnapshot(ws, correctedSnapshot as any, gameId, queryClient, {
        version: 2,
        viewerSeat: 1,
      })

      // Verify server correction applied
      await waitFor(() => {
        const finalState = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(gameId)
        )
        const finalSnapshot = selectSnapshot(finalState!)
        if (finalSnapshot.phase.phase === 'Bidding') {
          expect(finalSnapshot.phase.data.bids[1]).toBe(6) // Corrected value
        }
      })
    })
  })

  describe('Trump Selection Cycle', () => {
    it('completes trump selection with optimistic update and WS reconciliation', async () => {
      const gameId = 1

      // Create trump selection state
      const trumpSelectSnapshot = {
        ...biddingSnapshotFixture,
        phase: {
          phase: 'TrumpSelect' as const,
          data: {
            round: {
              hand_size: 8,
              leader: 0,
              bid_winner: 1,
              trump: null,
              tricks_won: [0, 0, 0, 0],
              bids: [2, 5, 3, 4],
            },
            to_act: 1,
            allowed_trumps: [
              'CLUBS',
              'DIAMONDS',
              'HEARTS',
              'SPADES',
              'NO_TRUMPS',
            ],
          },
        },
      }

      const initialState = createInitialStateWithVersion(gameId, 1, {
        viewerSeat: 1,
      })
      initialState.game = trumpSelectSnapshot as any

      queryClient.setQueryData(queryKeys.games.state(gameId), initialState)

      const { result: _syncResult } = renderHook(
        () => useGameSync({ initialState, gameId }),
        { queryClient }
      )

      const ws = await waitForWebSocketConnection()

      const { result: mutationResult } = renderHook(() => useSelectTrump(), {
        queryClient,
      })

      mocks.selectTrumpAction.mockResolvedValue({ kind: 'ok' })

      // Submit trump selection
      await act(async () => {
        await mutationResult.current.mutateAsync({
          gameId,
          trump: 'HEARTS',
          version: 1,
        })
      })

      // Verify optimistic update
      const optimisticState = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(gameId)
      )
      expect(optimisticState!.source).toBe('optimistic')
      const snapshot = selectSnapshot(optimisticState!)
      if (snapshot.phase.phase === 'TrumpSelect') {
        expect(snapshot.phase.data.round.trump).toBe('HEARTS')
      } else {
        throw new Error('Expected TrumpSelect phase')
      }

      // Simulate WS confirmation
      const updatedSnapshot = {
        ...trumpSelectSnapshot,
        phase: {
          ...trumpSelectSnapshot.phase,
          data: {
            ...(trumpSelectSnapshot.phase.phase === 'TrumpSelect'
              ? trumpSelectSnapshot.phase.data
              : {}),
            round: {
              ...(trumpSelectSnapshot.phase.phase === 'TrumpSelect'
                ? trumpSelectSnapshot.phase.data.round
                : {}),
              trump: 'HEARTS',
            },
          },
        },
      }

      sendWebSocketSnapshot(ws, updatedSnapshot as any, gameId, queryClient, {
        version: 2,
        viewerSeat: 1,
      })

      // Verify final state
      await waitFor(() => {
        const finalState = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(gameId)
        )
        expect(selectVersion(finalState!)).toBe(2)
        expect(finalState!.source).toBe('ws')
      })
    })
  })

  describe('Card Play Cycle', () => {
    it('completes card play with optimistic update and WS reconciliation', async () => {
      const gameId = 1
      const initialState = createInitialStateWithVersion(gameId, 1, {
        viewerSeat: 0,
        viewerHand: ['2H', 'KD', 'QC'],
      })
      initialState.game = trickSnapshotFixture

      queryClient.setQueryData(queryKeys.games.state(gameId), initialState)

      const { result: _syncResult } = renderHook(
        () => useGameSync({ initialState, gameId }),
        { queryClient }
      )

      const ws = await waitForWebSocketConnection()

      const { result: mutationResult } = renderHook(() => useSubmitPlay(), {
        queryClient,
      })

      mocks.submitPlayAction.mockResolvedValue({ kind: 'ok' })

      // Play card
      await act(async () => {
        await mutationResult.current.mutateAsync({
          gameId,
          card: '2H',
          version: 1,
        })
      })

      // Verify optimistic update - card added to current trick
      const optimisticState = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(gameId)
      )
      expect(optimisticState!.source).toBe('optimistic')
      const snapshot = selectSnapshot(optimisticState!)
      if (snapshot.phase.phase === 'Trick') {
        expect(snapshot.phase.data.current_trick).toHaveLength(3) // Was 2, now 3
        expect(snapshot.phase.data.current_trick[2]).toEqual([0, '2H'])
      } else {
        throw new Error('Expected Trick phase')
      }

      // Verify viewer hand NOT updated by optimistic mutation
      // (backend is authoritative for hand updates)
      expect(selectViewerHand(optimisticState!)).toEqual(['2H', 'KD', 'QC'])

      // Simulate WS state - hand updated by server
      const updatedSnapshot = {
        ...trickSnapshotFixture,
        phase: {
          ...trickSnapshotFixture.phase,
          data: {
            ...(trickSnapshotFixture.phase.phase === 'Trick'
              ? trickSnapshotFixture.phase.data
              : {}),
            current_trick: [
              [2, 'AS'],
              [3, 'TH'],
              [0, '2H'],
            ],
            to_act: 1,
          },
        },
      }

      sendWebSocketSnapshot(ws, updatedSnapshot as any, gameId, queryClient, {
        version: 2,
        viewerSeat: 0,
        viewerHand: ['KD', 'QC'], // Card removed from hand
      })

      // Verify final state
      await waitFor(() => {
        const finalState = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(gameId)
        )
        expect(selectVersion(finalState!)).toBe(2)
        expect(finalState!.source).toBe('ws')
        expect(selectViewerHand(finalState!)).toEqual(['KD', 'QC'])
      })
    })
  })
})
