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
  })

  it.each<RealtimeScenario>([
    {
      name: 'game_state (current game, newer version) updates snapshot + invalidates waitingLongest',
      gameId: 42,
      initialData: createInitialDataWithVersion(42, 1),
      seed: {
        snapshot: { gameId: 42, payload: createInitialDataWithVersion(42, 1) },
      },
      msg: gameStateMsg({ gameId: 42, version: 2 }),
      expect: { waitingLongestInvalidated: true, snapshotVersion: 2 },
    },
    {
      name: 'game_state (other game) does nothing',
      gameId: 42,
      initialData: createInitialDataWithVersion(42, 1),
      seed: {
        snapshot: { gameId: 42, payload: createInitialDataWithVersion(42, 1) },
      },
      msg: gameStateMsg({ gameId: 99, version: 2 }),
      expect: { waitingLongestInvalidated: false, snapshotVersion: 1 },
    },
    {
      name: 'game_state (current game, older/equal version) is ignored',
      gameId: 42,
      initialData: createInitialDataWithVersion(42, 1),
      seed: {
        snapshot: { gameId: 42, payload: createInitialDataWithVersion(42, 5) },
      },
      msg: gameStateMsg({ gameId: 42, version: 5 }),
      expect: { waitingLongestInvalidated: false, snapshotVersion: 5 },
    },
    {
      name: 'your_turn invalidates waitingLongest only',
      gameId: 42,
      initialData: createInitialDataWithVersion(42, 1),
      seed: {
        snapshot: { gameId: 42, payload: createInitialDataWithVersion(42, 1) },
      },
      msg: { type: 'your_turn', game_id: 42, version: 2 },
      expect: { waitingLongestInvalidated: true, snapshotVersion: 1 },
    },
    {
      name: 'long_wait_invalidated invalidates waitingLongest only',
      gameId: 42,
      initialData: createInitialDataWithVersion(42, 1),
      seed: {
        snapshot: { gameId: 42, payload: createInitialDataWithVersion(42, 1) },
      },
      msg: { type: 'long_wait_invalidated', game_id: 42 },
      expect: { waitingLongestInvalidated: true, snapshotVersion: 1 },
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

    // Spy on invalidation (source of truth for "waitingLongest changed" signals).
    const invalidateSpy = vi.spyOn(queryClient, 'invalidateQueries')

    await runRealtimeScenario(scenario, { queryClient, ws, invalidateSpy })

    // Sanity: snapshot cache key should be present if we asserted a version.
    if (scenario.expect.snapshotVersion !== undefined) {
      const cached = queryClient.getQueryData<GameRoomSnapshotPayload>(
        queryKeys.games.snapshot(scenario.gameId)
      )
      expect(cached?.version).toBe(scenario.expect.snapshotVersion)
    }
  })
})
