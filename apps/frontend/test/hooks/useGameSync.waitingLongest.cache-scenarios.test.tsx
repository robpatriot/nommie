import { beforeEach, describe, expect, it, vi } from 'vitest'
import { QueryClient } from '@tanstack/react-query'

import { useGameSync } from '@/hooks/useGameSync'
import { queryKeys } from '@/lib/queries/query-keys'
import { initSnapshotFixture } from '../mocks/game-snapshot'
import { createInitialDataWithVersion } from '../setup/game-room-client-helpers'
import { setupFetchMock } from '../setup/game-room-client-mocks'
import {
  MockWebSocket,
  mockWebSocketInstances,
} from '@/test/setup/mock-websocket'
import { renderHook, waitFor } from '../utils'

import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import type { WireMsg } from '@/lib/game-room/protocol/types'
import {
  runRealtimeScenario,
  type RealtimeScenario,
} from '../setup/waitingLongest-cache-scenarios'

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

// Ensure WebSocket is mocked for provider + hooks.
vi.stubGlobal('WebSocket', MockWebSocket)

// WebSocket config mocks (avoid env-validation coupling)
vi.mock('@/lib/config/env-validation', () => ({
  resolveWebSocketUrl: () => 'ws://localhost:3001',
  validateWebSocketConfig: () => {},
}))

// Mock error logger to avoid console noise
vi.mock('@/lib/logging/error-logger', () => ({
  logError: vi.fn(),
}))

function gameStateMsg(opts: { gameId: number; version: number }): WireMsg {
  return {
    type: 'game_state',
    topic: { kind: 'game', id: opts.gameId },
    version: opts.version,
    game: initSnapshotFixture,
    viewer: {
      seat: 0,
      hand: [],
      bidConstraints: null,
    },
  }
}

describe('useGameSync waitingLongest cache scenarios', () => {
  let queryClient: QueryClient

  beforeEach(() => {
    // Our scenarios intentionally seed query cache state without mounting useQuery observers.
    // Use a non-zero gcTime so seeded cache isn't immediately garbage-collected.
    queryClient = new QueryClient({
      defaultOptions: {
        queries: {
          retry: false,
          gcTime: Infinity,
        },
        mutations: {
          retry: false,
        },
      },
    })
    mockWebSocketInstances.length = 0
    vi.clearAllMocks()

    // Mock fetch for /api/ws-token endpoint (used by WebSocketProvider)
    setupFetchMock(originalFetch)

    // Ensure WebSocket is mocked
    vi.stubGlobal('WebSocket', MockWebSocket)

    mocks.getWaitingLongestGameAction.mockResolvedValue({
      kind: 'ok',
      data: [],
    })
  })

  it.each<RealtimeScenario>([
    {
      name: 'game_state (current game, newer version) updates snapshot (no LW refetch)',
      gameId: 42,
      initialData: createInitialDataWithVersion(42, 1),
      seed: {
        snapshot: { gameId: 42, payload: createInitialDataWithVersion(42, 1) },
      },
      msg: gameStateMsg({ gameId: 42, version: 2 }),
      expect: { lwRefetchCalls: 0, snapshotVersion: 2 },
    },
    {
      name: 'game_state (other game) does nothing',
      gameId: 42,
      initialData: createInitialDataWithVersion(42, 1),
      seed: {
        snapshot: { gameId: 42, payload: createInitialDataWithVersion(42, 1) },
      },
      msg: gameStateMsg({ gameId: 99, version: 2 }),
      expect: { lwRefetchCalls: 0, snapshotVersion: 1 },
    },
    {
      name: 'game_state (current game, older/equal version) is ignored',
      gameId: 42,
      initialData: createInitialDataWithVersion(42, 1),
      seed: {
        snapshot: { gameId: 42, payload: createInitialDataWithVersion(42, 5) },
      },
      msg: gameStateMsg({ gameId: 42, version: 5 }),
      expect: { lwRefetchCalls: 0, snapshotVersion: 5 },
    },
    {
      name: 'your_turn updates LW cache (no refetch when pool is small)',
      gameId: 42,
      initialData: createInitialDataWithVersion(42, 1),
      seed: {
        snapshot: { gameId: 42, payload: createInitialDataWithVersion(42, 1) },
        lwCache: {
          pool: [],
          isCompleteFromServer: false,
        },
      },
      msg: { type: 'your_turn', game_id: 42, version: 2 },
      expect: {
        lwRefetchCalls: 0,
        lwPoolAfterRefetch: [42],
        lwSnapshotGameIdAfter: null,
        snapshotVersion: 1,
      },
    },
    {
      name: 'long_wait_invalidated triggers LW refetch and updates pool',
      gameId: 42,
      initialData: createInitialDataWithVersion(42, 1),
      seed: {
        snapshot: { gameId: 42, payload: createInitialDataWithVersion(42, 1) },
      },
      msg: { type: 'long_wait_invalidated', game_id: 42 },
      expect: {
        lwRefetchCalls: 1,
        lwPoolAfterRefetch: [101, 202, 303],
        lwSnapshotGameIdAfter: null,
        snapshotVersion: 1,
      },
    },

    {
      name: 'your_turn (pool full, missing game) refetches and creates snapshot tied to that game',
      gameId: 42,
      initialData: createInitialDataWithVersion(42, 1),
      seed: {
        snapshot: { gameId: 42, payload: createInitialDataWithVersion(42, 1) },
        lwCache: {
          pool: [10, 11],
          isCompleteFromServer: false,
        },
      },
      msg: { type: 'your_turn', game_id: 99, version: 2 },
      expect: {
        lwRefetchCalls: 1,
        lwPoolAfterRefetch: [401, 402, 403],
        lwSnapshotGameIdAfter: 99,
        snapshotVersion: 1,
      },
    },

    {
      name: 'your_turn (snapshot for same game) restores snapshot with no refetch',
      gameId: 42,
      initialData: createInitialDataWithVersion(42, 1),
      seed: {
        snapshot: { gameId: 42, payload: createInitialDataWithVersion(42, 1) },
        lwCache: {
          pool: [10, 11],
          isCompleteFromServer: false,
          snapshot: {
            gameId: 99,
            pool: [501, 502],
            isCompleteFromServer: true,
          },
        },
      },
      msg: { type: 'your_turn', game_id: 99, version: 2 },
      expect: {
        lwRefetchCalls: 0,
        lwPoolAfterRefetch: [501, 502],
        lwSnapshotGameIdAfter: 99,
        snapshotVersion: 1,
      },
    },
  ])('$name', async (scenario) => {
    // Mount the hook (WebSocketProvider auto-connects when authenticated).
    renderHook(
      () =>
        useGameSync({
          initialData: scenario.initialData,
          gameId: scenario.gameId,
        }),
      {
        queryClient,
      }
    )

    // Wait for ws connection to appear.
    await waitFor(() => {
      expect(mockWebSocketInstances.length).toBe(1)
    })
    const ws = mockWebSocketInstances[0]

    await runRealtimeScenario(scenario, {
      queryClient,
      ws,
      clearLwRefetchMock: () => mocks.getWaitingLongestGameAction.mockClear(),
      getLwRefetchMockCallCount: () =>
        mocks.getWaitingLongestGameAction.mock.calls.length,
      setLwRefetchMockResponse: (pool) => {
        mocks.getWaitingLongestGameAction.mockResolvedValue({
          kind: 'ok',
          data: pool,
        })
      },
    })

    // Sanity: snapshot cache key should be present if we asserted a version.
    if (scenario.expect.snapshotVersion !== undefined) {
      const cached = queryClient.getQueryData<GameRoomSnapshotPayload>(
        queryKeys.games.snapshot(scenario.gameId)
      )
      expect(cached?.version).toBe(scenario.expect.snapshotVersion)
    }
  })
})
