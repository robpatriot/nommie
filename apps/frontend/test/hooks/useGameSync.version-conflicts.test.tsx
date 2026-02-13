// ============================================================================
// FILE: apps/frontend/test/hooks/useGameSync.version-conflicts.test.tsx
// Version conflict edge case tests validating monotonic version gating
// behavior in various edge scenarios.
// ============================================================================

import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, waitFor, act, createTestQueryClient } from '../utils'
import type { QueryClient } from '@tanstack/react-query'
import { useGameSync } from '@/hooks/useGameSync'
import { queryKeys } from '@/lib/queries/query-keys'
import { mockGetGameRoomStateAction } from '../../setupGameRoomActionsMock'
import {
  MockWebSocket,
  mockWebSocketInstances,
} from '@/test/setup/mock-websocket'
import { setupFetchMock } from '../setup/game-room-client-mocks'
import type { GameRoomState } from '@/lib/game-room/state'
import { selectVersion } from '@/lib/game-room/state'
import {
  createInitialStateWithVersion,
  waitForWebSocketConnection,
} from '../setup/game-room-client-helpers'
import { initSnapshotFixture } from '../mocks/game-snapshot'

const mocks = vi.hoisted(() => ({
  getWaitingLongestGameAction: vi.fn(),
}))

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
  resolveWebSocketUrl: () => 'ws://localhost:3001',
  validateWebSocketConfig: () => {},
}))

// Mock error logger to avoid console noise
vi.mock('@/lib/logging/error-logger', () => ({
  logError: vi.fn(),
}))

/**
 * Test helper to send a game_state message via WebSocket
 */
function serverSendGameState(
  ws: MockWebSocket,
  opts: {
    gameId: number
    version: number
    roundNo?: number
  }
) {
  act(() => {
    ws.onmessage?.(
      new MessageEvent('message', {
        data: JSON.stringify({
          type: 'game_state',
          topic: { kind: 'game', id: opts.gameId },
          version: opts.version,
          game: {
            ...initSnapshotFixture,
            game: {
              ...initSnapshotFixture.game,
              round_no: opts.roundNo ?? 0,
            },
          },
          viewer: {
            seat: 0,
            hand: [],
            bidConstraints: null,
          },
        }),
      })
    )
  })
}

describe('Version Conflict Edge Cases', () => {
  let queryClient: QueryClient

  beforeEach(() => {
    queryClient = createTestQueryClient()

    mockWebSocketInstances.length = 0
    vi.clearAllMocks()
    vi.useRealTimers()

    // Mock fetch for /api/ws-token endpoint
    setupFetchMock(originalFetch)

    vi.stubGlobal('WebSocket', MockWebSocket)

    mocks.getWaitingLongestGameAction.mockResolvedValue({
      kind: 'ok',
      data: [],
    })

    mockGetGameRoomStateAction.mockResolvedValue({
      kind: 'ok',
      data: createInitialStateWithVersion(1, 1),
    })

    process.env.NEXT_PUBLIC_BACKEND_BASE_URL = 'http://localhost:3001'
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

  describe('Same Version Conflicts', () => {
    it('WS message is rejected when optimistic and WS have same version', async () => {
      const gameId = 1
      const initialState = createInitialStateWithVersion(gameId, 5)

      // Seed cache with version 5
      queryClient.setQueryData(queryKeys.games.state(gameId), initialState)

      renderHook(() => useGameSync({ initialState, gameId }), { queryClient })

      const ws = await waitForWebSocketConnection()

      // Manually apply optimistic update with version 6
      const optimisticState: GameRoomState = {
        ...initialState,
        version: 6,
        source: 'optimistic',
        game: {
          ...initialState.game,
          game: { ...initialState.game.game, round_no: 99 },
        },
      }
      queryClient.setQueryData(queryKeys.games.state(gameId), optimisticState)

      // Send WS message with same version 6, but different data
      serverSendGameState(ws, { gameId, version: 6, roundNo: 42 })

      // Verify WS rejected (current.version >= incoming.version => reject)
      await waitFor(() => {
        const state = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(gameId)
        )
        expect(selectVersion(state!)).toBe(6)
        expect(state!.source).toBe('optimistic') // Still optimistic
        expect(state!.game.game.round_no).toBe(99) // Optimistic value preserved
      })
    })

    it('later WS message with same version is rejected', async () => {
      const gameId = 1
      const initialState = createInitialStateWithVersion(gameId, 1)

      renderHook(() => useGameSync({ initialState, gameId }), { queryClient })

      const ws = await waitForWebSocketConnection()

      // Send first v10 message
      serverSendGameState(ws, { gameId, version: 10, roundNo: 1 })

      await waitFor(() => {
        const state = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(gameId)
        )
        expect(state!.game.game.round_no).toBe(1)
      })

      // Send second v10 message (duplicate)
      serverSendGameState(ws, { gameId, version: 10, roundNo: 2 })

      // Verify second message rejected (>= check rejects equality)
      await waitFor(() => {
        const state = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(gameId)
        )
        expect(state!.game.game.round_no).toBe(1) // Original value preserved
      })
    })
  })

  describe('Missing Version Handling', () => {
    it('accepts WS message when cache has undefined version', async () => {
      const gameId = 1
      const initialState = createInitialStateWithVersion(gameId, 1)

      // Create state with undefined version
      const stateWithoutVersion: GameRoomState = {
        ...initialState,
        version: undefined as any,
      }

      queryClient.setQueryData(
        queryKeys.games.state(gameId),
        stateWithoutVersion
      )

      renderHook(() => useGameSync({ initialState, gameId }), { queryClient })

      const ws = await waitForWebSocketConnection()

      // Send WS message with version 5
      serverSendGameState(ws, { gameId, version: 5, roundNo: 10 })

      // Verify message applied (undefined < 5)
      await waitFor(() => {
        const state = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(gameId)
        )
        expect(selectVersion(state!)).toBe(5)
        expect(state!.game.game.round_no).toBe(10)
      })
    })

    it('accepts WS message with undefined version when cache has version', async () => {
      const gameId = 1
      const initialState = createInitialStateWithVersion(gameId, 5)

      queryClient.setQueryData(queryKeys.games.state(gameId), initialState)

      renderHook(() => useGameSync({ initialState, gameId }), { queryClient })

      const ws = await waitForWebSocketConnection()

      // Send WS message with undefined version (malformed)
      act(() => {
        ws.onmessage?.(
          new MessageEvent('message', {
            data: JSON.stringify({
              type: 'game_state',
              topic: { kind: 'game', id: gameId },
              version: undefined, // Malformed
              game: {
                ...initSnapshotFixture,
                game: { ...initSnapshotFixture.game, round_no: 99 },
              },
              viewer: { seat: 0, hand: [], bidConstraints: null },
            }),
          })
        )
      })

      // Verify message accepted (check: 5 >= undefined is false, so applies)
      await waitFor(() => {
        const stateAfter = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(gameId)
        )
        expect(stateAfter!.game.game.round_no).toBe(99)
      })
    })

    it('handles both undefined gracefully (applies update)', async () => {
      const gameId = 1
      const initialState = createInitialStateWithVersion(gameId, 1)

      // Cache with undefined version
      const stateWithoutVersion: GameRoomState = {
        ...initialState,
        version: undefined as any,
      }

      queryClient.setQueryData(
        queryKeys.games.state(gameId),
        stateWithoutVersion
      )

      renderHook(() => useGameSync({ initialState, gameId }), { queryClient })

      const ws = await waitForWebSocketConnection()

      // Send message with undefined version
      act(() => {
        ws.onmessage?.(
          new MessageEvent('message', {
            data: JSON.stringify({
              type: 'game_state',
              topic: { kind: 'game', id: gameId },
              version: undefined,
              game: {
                ...initSnapshotFixture,
                game: { ...initSnapshotFixture.game, round_no: 88 },
              },
              viewer: { seat: 0, hand: [], bidConstraints: null },
            }),
          })
        )
      })

      // Verify applies (both undefined => check fails, so update happens)
      await waitFor(() => {
        const state = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(gameId)
        )
        // Check: current.version >= incoming.version
        // undefined >= undefined is false
        // So should NOT update... but let's verify actual behavior
        expect(state!.game.game.round_no).toBe(88)
      })
    })
  })

  describe('Version Boundary Cases', () => {
    it('handles version 0 correctly', async () => {
      const gameId = 1
      const initialState = createInitialStateWithVersion(gameId, 0)

      queryClient.setQueryData(queryKeys.games.state(gameId), initialState)

      renderHook(() => useGameSync({ initialState, gameId }), { queryClient })

      const ws = await waitForWebSocketConnection()

      // Send v1 message
      serverSendGameState(ws, { gameId, version: 1, roundNo: 5 })

      // Verify upgrade from 0 to 1
      await waitFor(() => {
        const state = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(gameId)
        )
        expect(selectVersion(state!)).toBe(1)
      })
    })

    it('handles large version numbers', async () => {
      const gameId = 1
      const initialState = createInitialStateWithVersion(gameId, 999999)

      queryClient.setQueryData(queryKeys.games.state(gameId), initialState)

      renderHook(() => useGameSync({ initialState, gameId }), { queryClient })

      const ws = await waitForWebSocketConnection()

      // Send even larger version
      serverSendGameState(ws, { gameId, version: 1000000, roundNo: 100 })

      await waitFor(() => {
        const state = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(gameId)
        )
        expect(selectVersion(state!)).toBe(1000000)
      })
    })

    it('monotonic check works with negative versions (edge case)', async () => {
      const gameId = 1
      const initialState = createInitialStateWithVersion(gameId, 5)

      // Manually set negative version (should never happen, but test robustness)
      initialState.version = -1 as any

      queryClient.setQueryData(queryKeys.games.state(gameId), initialState)

      renderHook(() => useGameSync({ initialState, gameId }), { queryClient })

      const ws = await waitForWebSocketConnection()

      // Send v0 message
      serverSendGameState(ws, { gameId, version: 0, roundNo: 1 })

      // Verify upgrade (0 > -1)
      await waitFor(() => {
        const state = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(gameId)
        )
        expect(selectVersion(state!)).toBe(0)
      })
    })
  })

  describe('Rapid Version Progression', () => {
    it('handles rapid version increments correctly', async () => {
      const gameId = 1
      const initialState = createInitialStateWithVersion(gameId, 1)

      renderHook(() => useGameSync({ initialState, gameId }), { queryClient })

      const ws = await waitForWebSocketConnection()

      // Send versions 2, 3, 4, 5 rapidly
      for (let v = 2; v <= 5; v++) {
        serverSendGameState(ws, { gameId, version: v, roundNo: v })
      }

      // Verify final version is 5
      await waitFor(() => {
        const state = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(gameId)
        )
        expect(selectVersion(state!)).toBe(5)
        expect(state!.game.game.round_no).toBe(5)
      })
    })

    it('ignores out-of-order messages in rapid sequence', async () => {
      const gameId = 1
      const initialState = createInitialStateWithVersion(gameId, 1)

      renderHook(() => useGameSync({ initialState, gameId }), { queryClient })

      const ws = await waitForWebSocketConnection()

      // Send v10 first
      serverSendGameState(ws, { gameId, version: 10, roundNo: 10 })

      await waitFor(() => {
        const state = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(gameId)
        )
        expect(selectVersion(state!)).toBe(10)
      })

      // Then send v5, v7, v12 (only v12 should apply)
      serverSendGameState(ws, { gameId, version: 5, roundNo: 5 })
      serverSendGameState(ws, { gameId, version: 7, roundNo: 7 })
      serverSendGameState(ws, { gameId, version: 12, roundNo: 12 })

      await waitFor(() => {
        const state = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(gameId)
        )
        expect(selectVersion(state!)).toBe(12)
        expect(state!.game.game.round_no).toBe(12)
      })
    })
  })
})
