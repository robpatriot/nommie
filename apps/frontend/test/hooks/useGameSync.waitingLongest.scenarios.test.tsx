import { beforeEach, describe, expect, it, vi } from 'vitest'
import { renderHook, waitFor, act } from '../utils'
import type { QueryClient } from '@tanstack/react-query'
import { QueryClient as RQQueryClient } from '@tanstack/react-query'

import { useGameSync } from '@/hooks/useGameSync'
import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import { queryKeys } from '@/lib/queries/query-keys'
import { initSnapshotFixture } from '../mocks/game-snapshot'
import { createInitialDataWithVersion } from '../setup/game-room-client-helpers'
import { setupFetchMock } from '../setup/game-room-client-mocks'
import {
  MockWebSocket,
  mockWebSocketInstances,
} from '@/test/setup/mock-websocket'
import { mockGetGameRoomSnapshotAction } from '../../setupGameRoomActionsMock'

// Track original fetch
const originalFetch = globalThis.fetch

// Ensure WebSocket is mocked
vi.stubGlobal('WebSocket', MockWebSocket)

// Mock WebSocket config validation (avoid env coupling)
vi.mock('@/lib/config/env-validation', () => ({
  resolveWebSocketUrl: () => 'ws://localhost:3001',
  validateWebSocketConfig: () => {},
}))

// Mock error logger to avoid console noise
vi.mock('@/lib/logging/error-logger', () => ({
  logError: vi.fn(),
}))

function serverSendJson(ws: MockWebSocket, msg: unknown) {
  ws.onmessage?.(
    new MessageEvent('message', {
      data: JSON.stringify(msg),
    })
  )
}

function getSentJson(ws: MockWebSocket): unknown[] {
  return ws.sent.map((s) => {
    try {
      return JSON.parse(s) as unknown
    } catch {
      return s
    }
  })
}

function findSentByType<T extends { type: string }>(
  ws: MockWebSocket,
  type: string
): T | undefined {
  return getSentJson(ws).find(
    (m): m is T =>
      typeof m === 'object' && m !== null && (m as any).type === type
  )
}

async function waitForWsCount(n: number) {
  await waitFor(() => {
    expect(mockWebSocketInstances.length).toBe(n)
  })
}

async function waitForSentType(ws: MockWebSocket, type: string) {
  await waitFor(() => {
    expect(findSentByType(ws, type)).toBeDefined()
  })
}

async function connectAndSubscribe(
  ws: MockWebSocket,
  opts: { expectedGameId: number; protocol?: number; userId?: number }
) {
  const protocol = opts.protocol ?? 1
  const userId = opts.userId ?? 123

  await waitForSentType(ws, 'hello')

  await act(async () => {
    serverSendJson(ws, { type: 'hello_ack', protocol, user_id: userId })
  })

  await waitForSentType(ws, 'subscribe')

  const subscribe = findSentByType<{
    type: 'subscribe'
    topic: { kind: 'game'; id: number }
  }>(ws, 'subscribe')

  expect(subscribe?.topic).toEqual({ kind: 'game', id: opts.expectedGameId })

  await act(async () => {
    serverSendJson(ws, { type: 'ack', message: 'subscribed' })
  })
}

type Scenario = {
  name: string
  gameId: number
  seed: {
    snapshot: GameRoomSnapshotPayload
    waitingLongest?: number[]
  }
  msg: unknown
  expect: {
    waitingLongestInvalidated: boolean
    snapshotVersion: number
  }
}

function gameStateMsg(opts: { gameId: number; version: number }): unknown {
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
    // Use non-zero gcTime so seeded cache isn’t immediately collected (we seed without observers).
    queryClient = new RQQueryClient({
      defaultOptions: {
        queries: { retry: false, gcTime: Infinity },
        mutations: { retry: false },
      },
    })

    mockWebSocketInstances.length = 0
    vi.clearAllMocks()

    setupFetchMock(originalFetch)
    vi.stubGlobal('WebSocket', MockWebSocket)

    // Default snapshot action mock (should not be called by these scenarios)
    mockGetGameRoomSnapshotAction.mockResolvedValue({
      kind: 'ok',
      data: createInitialDataWithVersion(0, 1),
    })
  })

  it.each<Scenario>([
    {
      name: 'game_state (current game, newer version) updates snapshot + invalidates waitingLongest',
      gameId: 42,
      seed: {
        snapshot: createInitialDataWithVersion(42, 1),
        waitingLongest: [42],
      },
      msg: gameStateMsg({ gameId: 42, version: 2 }),
      expect: { waitingLongestInvalidated: true, snapshotVersion: 2 },
    },
    {
      name: 'game_state (other game) does not touch snapshot or waitingLongest',
      gameId: 42,
      seed: {
        snapshot: createInitialDataWithVersion(42, 1),
        waitingLongest: [99],
      },
      msg: gameStateMsg({ gameId: 99, version: 2 }),
      expect: { waitingLongestInvalidated: false, snapshotVersion: 1 },
    },
    {
      name: 'game_state (current game, stale version) is ignored (no invalidation)',
      gameId: 42,
      seed: {
        snapshot: createInitialDataWithVersion(42, 5),
        waitingLongest: [42],
      },
      msg: gameStateMsg({ gameId: 42, version: 5 }),
      expect: { waitingLongestInvalidated: false, snapshotVersion: 5 },
    },
    {
      name: 'your_turn invalidates waitingLongest only',
      gameId: 42,
      seed: {
        snapshot: createInitialDataWithVersion(42, 1),
        waitingLongest: [42],
      },
      msg: { type: 'your_turn', game_id: 42, version: 2 },
      expect: { waitingLongestInvalidated: true, snapshotVersion: 1 },
    },
    {
      name: 'yourturn (legacy) invalidates waitingLongest only',
      gameId: 42,
      seed: {
        snapshot: createInitialDataWithVersion(42, 1),
        waitingLongest: [42],
      },
      msg: { type: 'yourturn', game_id: 42, version: 2 },
      expect: { waitingLongestInvalidated: true, snapshotVersion: 1 },
    },
    {
      name: 'long_wait_invalidated invalidates waitingLongest only',
      gameId: 42,
      seed: {
        snapshot: createInitialDataWithVersion(42, 1),
        waitingLongest: [42],
      },
      msg: { type: 'long_wait_invalidated', game_id: 42 },
      expect: { waitingLongestInvalidated: true, snapshotVersion: 1 },
    },
  ])('$name', async (scenario) => {
    const initialData = scenario.seed.snapshot

    renderHook(() => useGameSync({ initialData, gameId: scenario.gameId }), {
      queryClient,
    })

    await waitForWsCount(1)
    const ws = mockWebSocketInstances[0]
    await connectAndSubscribe(ws, { expectedGameId: scenario.gameId })

    // Seed cache state
    queryClient.setQueryData(
      queryKeys.games.snapshot(scenario.gameId),
      scenario.seed.snapshot
    )
    if (scenario.seed.waitingLongest) {
      queryClient.setQueryData(
        queryKeys.games.waitingLongest(),
        scenario.seed.waitingLongest
      )
    }

    const invalidateSpy = vi.spyOn(queryClient, 'invalidateQueries')

    act(() => {
      serverSendJson(ws, scenario.msg)
    })

    if (scenario.expect.waitingLongestInvalidated) {
      await waitFor(() => {
        expect(invalidateSpy).toHaveBeenCalledWith({
          queryKey: queryKeys.games.waitingLongest(),
        })
      })
    } else {
      await act(async () => {
        await Promise.resolve()
      })
      expect(invalidateSpy).not.toHaveBeenCalledWith({
        queryKey: queryKeys.games.waitingLongest(),
      })
    }

    await waitFor(() => {
      const cached = queryClient.getQueryData<GameRoomSnapshotPayload>(
        queryKeys.games.snapshot(scenario.gameId)
      )
      expect(cached?.version).toBe(scenario.expect.snapshotVersion)
    })
  })
})
