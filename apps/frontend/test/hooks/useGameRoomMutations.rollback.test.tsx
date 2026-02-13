// ============================================================================
// FILE: apps/frontend/test/hooks/useGameRoomMutations.rollback.test.tsx
// Optimistic update rollback tests validating error recovery and state
// restoration when mutations fail.
// ============================================================================

import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, waitFor, act, createTestQueryClient } from '../utils'
import type { QueryClient } from '@tanstack/react-query'
import {
  useSubmitBid,
  useSelectTrump,
  useSubmitPlay,
} from '@/hooks/mutations/useGameRoomMutations'
import { queryKeys } from '@/lib/queries/query-keys'
import type { GameRoomState } from '@/lib/game-room/state'
import { selectSnapshot } from '@/lib/game-room/state'
import { createInitialStateWithVersion } from '../setup/game-room-client-helpers'
import {
  biddingSnapshotFixture,
  trickSnapshotFixture,
} from '../mocks/game-snapshot'
import { BackendApiError } from '@/lib/api'

const mocks = vi.hoisted(() => ({
  submitBidAction: vi.fn(),
  selectTrumpAction: vi.fn(),
  submitPlayAction: vi.fn(),
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

describe('Optimistic Update Rollback Tests', () => {
  let queryClient: QueryClient

  beforeEach(() => {
    queryClient = createTestQueryClient()
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  describe('Bid Mutation Rollback', () => {
    it('rolls back optimistic bid when server returns conflict error', async () => {
      const gameId = 1
      const initialState = createInitialStateWithVersion(gameId, 1, {
        viewerSeat: 1,
      })
      initialState.game = biddingSnapshotFixture

      // Set initial cache
      queryClient.setQueryData(queryKeys.games.state(gameId), initialState)

      const { result } = renderHook(() => useSubmitBid(), { queryClient })

      // Capture initial state
      const stateBefore = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(gameId)
      )
      const snapshotBefore = selectSnapshot(stateBefore!)
      const bidsBefore =
        snapshotBefore.phase.phase === 'Bidding'
          ? snapshotBefore.phase.data.bids
          : []

      // Mock server returning conflict error
      const conflictError = new BackendApiError(
        'Another player bid first',
        409,
        'CONFLICT'
      )
      mocks.submitBidAction.mockRejectedValue(conflictError)

      // Attempt mutation (should fail)
      await act(async () => {
        try {
          await result.current.mutateAsync({
            gameId,
            bid: 5,
            version: 1,
          })
        } catch {
          // Expected to throw
        }
      })

      // Verify state rolled back to original
      await waitFor(() => {
        const stateAfter = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(gameId)
        )
        expect(stateAfter).toEqual(stateBefore)

        const snapshotAfter = selectSnapshot(stateAfter!)
        if (snapshotAfter.phase.phase === 'Bidding') {
          expect(snapshotAfter.phase.data.bids).toEqual(bidsBefore)
        }
      })

      // Verify no optimistic marker remains
      const finalState = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(gameId)
      )
      expect(finalState!.source).not.toBe('optimistic')
    })

    it('rolls back optimistic bid when server returns validation error', async () => {
      const gameId = 1
      const initialState = createInitialStateWithVersion(gameId, 1, {
        viewerSeat: 1,
      })
      initialState.game = biddingSnapshotFixture

      queryClient.setQueryData(queryKeys.games.state(gameId), initialState)

      const { result } = renderHook(() => useSubmitBid(), { queryClient })

      const stateBefore = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(gameId)
      )

      // Mock validation error (invalid bid value)
      mocks.submitBidAction.mockResolvedValue({
        kind: 'error',
        message: 'Bid exceeds maximum',
        status: 400,
        code: 'VALIDATION_ERROR',
      })

      await act(async () => {
        try {
          await result.current.mutateAsync({
            gameId,
            bid: 99,
            version: 1,
          })
        } catch {
          // Expected to throw
        }
      })

      // Verify rollback
      const stateAfter = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(gameId)
      )
      expect(stateAfter).toEqual(stateBefore)
    })

    it('preserves mutation pending state during rollback', async () => {
      const gameId = 1
      const initialState = createInitialStateWithVersion(gameId, 1, {
        viewerSeat: 1,
      })
      initialState.game = biddingSnapshotFixture

      queryClient.setQueryData(queryKeys.games.state(gameId), initialState)

      const { result } = renderHook(() => useSubmitBid(), { queryClient })

      // Mock slow error response
      mocks.submitBidAction.mockImplementation(
        () =>
          new Promise((_, reject) =>
            setTimeout(() => reject(new Error('Server error')), 100)
          )
      )

      // Start mutation
      const mutationPromise = act(async () => {
        try {
          await result.current.mutateAsync({
            gameId,
            bid: 5,
            version: 1,
          })
        } catch {
          // Expected
        }
      })

      // Verify pending during mutation
      await waitFor(() => {
        expect(result.current.isPending).toBe(true)
      })

      // Wait for completion
      await mutationPromise

      // Verify no longer pending after rollback
      await waitFor(() => {
        expect(result.current.isPending).toBe(false)
      })
    })
  })

  describe('Trump Selection Rollback', () => {
    it('rolls back optimistic trump selection on error', async () => {
      const gameId = 1

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

      const { result } = renderHook(() => useSelectTrump(), { queryClient })

      const stateBefore = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(gameId)
      )
      const snapshotBefore = selectSnapshot(stateBefore!)
      const trumpBefore =
        snapshotBefore.phase.phase === 'TrumpSelect'
          ? snapshotBefore.phase.data.round.trump
          : null

      // Mock error
      mocks.selectTrumpAction.mockResolvedValue({
        kind: 'error',
        message: 'Not your turn',
        status: 403,
        code: 'FORBIDDEN',
      })

      await act(async () => {
        try {
          await result.current.mutateAsync({
            gameId,
            trump: 'HEARTS',
            version: 1,
          })
        } catch {
          // Expected
        }
      })

      // Verify rollback
      const stateAfter = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(gameId)
      )
      const snapshotAfter = selectSnapshot(stateAfter!)
      if (snapshotAfter.phase.phase === 'TrumpSelect') {
        expect(snapshotAfter.phase.data.round.trump).toBe(trumpBefore)
      }
      expect(stateAfter).toEqual(stateBefore)
    })
  })

  describe('Card Play Rollback', () => {
    it('rolls back optimistic card play when server returns invalid card error', async () => {
      const gameId = 1
      const initialState = createInitialStateWithVersion(gameId, 1, {
        viewerSeat: 0,
        viewerHand: ['2H', 'KD', 'QC'],
      })
      initialState.game = trickSnapshotFixture

      queryClient.setQueryData(queryKeys.games.state(gameId), initialState)

      const { result } = renderHook(() => useSubmitPlay(), { queryClient })

      const stateBefore = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(gameId)
      )
      const snapshotBefore = selectSnapshot(stateBefore!)
      const trickBefore =
        snapshotBefore.phase.phase === 'Trick'
          ? snapshotBefore.phase.data.current_trick
          : []

      // Mock error (card not playable due to suit rules)
      mocks.submitPlayAction.mockResolvedValue({
        kind: 'error',
        message: 'Card violates suit rules',
        status: 400,
        code: 'INVALID_CARD',
      })

      await act(async () => {
        try {
          await result.current.mutateAsync({
            gameId,
            card: 'KD',
            version: 1,
          })
        } catch {
          // Expected
        }
      })

      // Verify trick rolled back (card not added)
      const stateAfter = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(gameId)
      )
      const snapshotAfter = selectSnapshot(stateAfter!)
      if (snapshotAfter.phase.phase === 'Trick') {
        expect(snapshotAfter.phase.data.current_trick).toEqual(trickBefore)
      }
      expect(stateAfter).toEqual(stateBefore)
    })

    it('rolls back when network error occurs', async () => {
      const gameId = 1
      const initialState = createInitialStateWithVersion(gameId, 1, {
        viewerSeat: 0,
        viewerHand: ['2H', 'KD', 'QC'],
      })
      initialState.game = trickSnapshotFixture

      queryClient.setQueryData(queryKeys.games.state(gameId), initialState)

      const { result } = renderHook(() => useSubmitPlay(), { queryClient })

      const stateBefore = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(gameId)
      )

      // Mock network error
      mocks.submitPlayAction.mockRejectedValue(
        new Error('Network request failed')
      )

      await act(async () => {
        try {
          await result.current.mutateAsync({
            gameId,
            card: '2H',
            version: 1,
          })
        } catch {
          // Expected
        }
      })

      // Verify complete rollback
      const stateAfter = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(gameId)
      )
      expect(stateAfter).toEqual(stateBefore)
    })
  })

  describe('Edge Cases', () => {
    it('handles rollback when cache is cleared during mutation', async () => {
      const gameId = 1
      const initialState = createInitialStateWithVersion(gameId, 1, {
        viewerSeat: 1,
      })
      initialState.game = biddingSnapshotFixture

      queryClient.setQueryData(queryKeys.games.state(gameId), initialState)

      const { result } = renderHook(() => useSubmitBid(), { queryClient })

      // Mock error
      mocks.submitBidAction.mockResolvedValue({
        kind: 'error',
        message: 'Error',
        status: 500,
      })

      // Clear cache during mutation
      await act(async () => {
        queryClient.removeQueries({ queryKey: queryKeys.games.state(gameId) })

        try {
          await result.current.mutateAsync({
            gameId,
            bid: 5,
            version: 1,
          })
        } catch {
          // Expected
        }
      })

      // Verify no crash - cache remains empty
      const stateAfter = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(gameId)
      )
      expect(stateAfter).toBeUndefined()
    })

    it('handles rollback when previousState was undefined', async () => {
      const gameId = 1

      const { result } = renderHook(() => useSubmitBid(), { queryClient })

      // No initial state in cache
      mocks.submitBidAction.mockResolvedValue({
        kind: 'error',
        message: 'Error',
        status: 500,
      })

      await act(async () => {
        try {
          await result.current.mutateAsync({
            gameId,
            bid: 5,
            version: 1,
          })
        } catch {
          // Expected
        }
      })

      // Verify no crash
      const stateAfter = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(gameId)
      )
      expect(stateAfter).toBeUndefined()
    })
  })
})
