// ============================================================================
// FILE: apps/frontend/test/hooks/useGameSync.test.tsx
// ============================================================================

import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, waitFor, act, createTestQueryClient } from '../utils'
import type { QueryClient } from '@tanstack/react-query'
import { useGameSync } from '@/hooks/useGameSync'
import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import { initSnapshotFixture } from '../mocks/game-snapshot'
import { queryKeys } from '@/lib/queries/query-keys'
import { mockGetGameRoomSnapshotAction } from '../../setupGameRoomActionsMock'
import {
  MockWebSocket,
  mockWebSocketInstances,
} from '@/test/setup/mock-websocket'
import { setupFetchMock } from '../setup/game-room-client-mocks'
import {
  createInitialData,
  createInitialDataWithVersion,
} from '../setup/game-room-client-helpers'

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
 * Test helpers (server-side simulation)
 */
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

/**
 * Establishes provider hello handshake (hello -> hello_ack) and then waits for
 * the hook to subscribe (subscribe -> ack).
 *
 * This matches the post-refactor world:
 * - WebSocketProvider owns the socket + hello/hello_ack gating
 * - useGameSync issues subscribe/unsubscribe for the current gameId
 */
async function connectAndSubscribe(
  ws: MockWebSocket,
  opts: { expectedGameId: number; protocol?: number; userId?: number } = {
    expectedGameId: 1,
  }
) {
  const protocol = opts.protocol ?? 1
  const userId = opts.userId ?? 123

  // Provider sends hello on open
  await waitForSentType(ws, 'hello')

  // Server acks hello -> provider becomes connected
  await act(async () => {
    serverSendJson(ws, { type: 'hello_ack', protocol, user_id: userId })
  })

  // Hook should subscribe to current game
  await waitForSentType(ws, 'subscribe')

  const subscribe = findSentByType<{
    type: 'subscribe'
    topic: { kind: 'game'; id: number }
  }>(ws, 'subscribe')

  expect(subscribe?.topic).toEqual({ kind: 'game', id: opts.expectedGameId })

  // Server acks subscription
  await act(async () => {
    serverSendJson(ws, { type: 'ack', message: 'subscribed' })
  })
}

describe('useGameSync', () => {
  let queryClient: QueryClient

  beforeEach(() => {
    queryClient = createTestQueryClient()

    mockWebSocketInstances.length = 0
    vi.clearAllMocks()
    vi.useRealTimers()

    // Mock fetch for /api/ws-token endpoint (used by WebSocketProvider)
    setupFetchMock(originalFetch)

    // Ensure WebSocket is mocked
    vi.stubGlobal('WebSocket', MockWebSocket)

    mockGetGameRoomSnapshotAction.mockImplementation(
      async ({ gameId }: { gameId: number }) => ({
        kind: 'ok' as const,
        data: createInitialDataWithVersion(gameId),
      })
    )

    // Used by env-validation mock / resolveWebSocketUrl in some codepaths
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

  describe('Subscription + Version gating', () => {
    it('subscribes on mount and reports connected after hello_ack', async () => {
      const initialData = createInitialDataWithVersion(1)

      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        { queryClient }
      )

      await waitForWsCount(1)
      const ws = mockWebSocketInstances[0]
      expect(ws.readyState).toBe(MockWebSocket.OPEN)

      await connectAndSubscribe(ws, { expectedGameId: 1 })

      await waitFor(() => {
        expect(result.current.connectionState).toBe('connected')
      })
    })

    it('does not create a new socket when gameId changes; it re-subscribes and resets version gate', async () => {
      const initialData1 = createInitialData()
      initialData1.version = 1
      initialData1.etag = 'etag-1'

      const initialData2 = createInitialData()
      initialData2.version = 1
      initialData2.etag = 'etag-2'

      const { rerender } = renderHook(
        ({ gameId, initialData }) => useGameSync({ initialData, gameId }),
        {
          queryClient,
          initialProps: { gameId: 1, initialData: initialData1 },
        }
      )

      await waitForWsCount(1)
      const ws = mockWebSocketInstances[0]
      await connectAndSubscribe(ws, { expectedGameId: 1 })

      // Apply high-version game_state for game 1 (sets last seen high)
      act(() => {
        serverSendJson(ws, {
          type: 'game_state',
          topic: { kind: 'game', id: 1 },
          version: 100,
          game: initialData1.snapshot,
          viewer: {
            seat: initialData1.viewerSeat ?? null,
            hand: [],
            bidConstraints: null,
          },
        })
      })

      // Change gameId WITHOUT unmounting.
      await act(async () => {
        rerender({ gameId: 2, initialData: initialData2 })
      })

      // Still exactly one socket (WS is visit-lifetime).
      expect(mockWebSocketInstances.length).toBe(1)
      expect(ws.readyState).toBe(MockWebSocket.OPEN)

      // A new subscribe should be sent for game 2.
      await waitFor(() => {
        const sent = getSentJson(ws)
        const subscribes = sent.filter(
          (m) =>
            typeof m === 'object' &&
            m !== null &&
            (m as any).type === 'subscribe'
        )
        expect(subscribes.length).toBeGreaterThanOrEqual(2)
      })

      const lastSubscribe = (() => {
        const subscribes = getSentJson(ws).filter(
          (m) =>
            typeof m === 'object' &&
            m !== null &&
            (m as any).type === 'subscribe'
        ) as Array<{ type: 'subscribe'; topic: { kind: 'game'; id: number } }>
        return subscribes[subscribes.length - 1]
      })()

      expect(lastSubscribe.topic).toEqual({ kind: 'game', id: 2 })

      // Server acks subscription (if your hook expects it)
      await act(async () => {
        serverSendJson(ws, { type: 'ack', message: 'subscribed' })
      })

      // Send low-version game_state for game 2 (must NOT be dropped due to old last=100 from game 1).
      act(() => {
        serverSendJson(ws, {
          type: 'game_state',
          topic: { kind: 'game', id: 2 },
          version: 2,
          game: initialData2.snapshot,
          viewer: {
            seat: initialData2.viewerSeat ?? null,
            hand: [],
            bidConstraints: null,
          },
        })
      })

      await waitFor(() => {
        const data = queryClient.getQueryData(queryKeys.games.snapshot(2))
        expect(data).toBeTruthy()
      })
    })

    it('does not re-subscribe when initialData changes but gameId stays the same', async () => {
      const initialDataV1 = createInitialDataWithVersion(1, 1)
      const initialDataV2 = createInitialDataWithVersion(1, 2)

      const { rerender } = renderHook(
        ({ gameId, initialData }) => useGameSync({ initialData, gameId }),
        {
          queryClient,
          initialProps: { gameId: 1, initialData: initialDataV1 },
        }
      )

      await waitForWsCount(1)
      const ws = mockWebSocketInstances[0]
      await connectAndSubscribe(ws, { expectedGameId: 1 })

      const subscribeCountBefore = getSentJson(ws).filter(
        (m) =>
          typeof m === 'object' && m !== null && (m as any).type === 'subscribe'
      ).length

      // Rerender with SAME gameId but NEW initialData (new etag + version)
      rerender({ gameId: 1, initialData: initialDataV2 })

      // No new subscribe should be sent
      await waitFor(() => {
        const subscribeCountAfter = getSentJson(ws).filter(
          (m) =>
            typeof m === 'object' &&
            m !== null &&
            (m as any).type === 'subscribe'
        ).length
        expect(subscribeCountAfter).toBe(subscribeCountBefore)
      })
    })
  })

  describe('WebSocket message handling', () => {
    it('updates query cache when receiving game_state', async () => {
      const initialData = createInitialDataWithVersion(1, 1)

      renderHook(() => useGameSync({ initialData, gameId: 1 }), { queryClient })

      await waitForWsCount(1)
      const ws = mockWebSocketInstances[0]
      await connectAndSubscribe(ws, { expectedGameId: 1 })

      const updatedSnapshot = {
        ...initSnapshotFixture,
        game: { ...initSnapshotFixture.game, round_no: 2 },
      }

      act(() => {
        serverSendJson(ws, {
          type: 'game_state',
          topic: { kind: 'game', id: 1 },
          version: 2,
          game: updatedSnapshot,
          viewer: {
            seat: 0,
            hand: ['2H', '3C'],
            bidConstraints: null,
          },
        })
      })

      await waitFor(() => {
        const cached = queryClient.getQueryData<GameRoomSnapshotPayload>(
          queryKeys.games.snapshot(1)
        )
        expect(cached).toBeDefined()
        expect(cached?.version).toBe(2)
        expect(cached?.snapshot.game.round_no).toBe(2)
        expect(cached?.viewerHand).toEqual(['2H', '3C'])
      })
    })

    it('ignores stale game_state and applies newer ones', async () => {
      const initialDataV1 = createInitialDataWithVersion(1, 1)

      const { rerender } = renderHook(
        ({ initialData }) => useGameSync({ initialData, gameId: 1 }),
        {
          queryClient,
          initialProps: { initialData: initialDataV1 },
        }
      )

      await waitForWsCount(1)
      const ws = mockWebSocketInstances[0]
      await connectAndSubscribe(ws, { expectedGameId: 1 })

      // Establish version 5
      act(() => {
        serverSendJson(ws, {
          type: 'game_state',
          topic: { kind: 'game', id: 1 },
          version: 5,
          game: initSnapshotFixture,
          viewer: { seat: 0, hand: [], bidConstraints: null },
        })
      })

      await waitFor(() => {
        const cached = queryClient.getQueryData<GameRoomSnapshotPayload>(
          queryKeys.games.snapshot(1)
        )
        expect(cached?.version).toBe(5)
      })

      const before = queryClient.getQueryData<GameRoomSnapshotPayload>(
        queryKeys.games.snapshot(1)
      )

      // Send stale v3 (different content) -> should not change cache
      act(() => {
        serverSendJson(ws, {
          type: 'game_state',
          topic: { kind: 'game', id: 1 },
          version: 3,
          game: {
            ...initSnapshotFixture,
            game: { ...initSnapshotFixture.game, round_no: 99 },
          },
          viewer: { seat: 0, hand: ['KS'], bidConstraints: null },
        })
      })

      await waitFor(() => {
        const afterStale = queryClient.getQueryData<GameRoomSnapshotPayload>(
          queryKeys.games.snapshot(1)
        )
        expect(afterStale).toEqual(before)
      })

      // Send fresh v6 -> should apply
      act(() => {
        serverSendJson(ws, {
          type: 'game_state',
          topic: { kind: 'game', id: 1 },
          version: 6,
          game: {
            ...initSnapshotFixture,
            game: { ...initSnapshotFixture.game, round_no: 2 },
          },
          viewer: { seat: 0, hand: ['AS'], bidConstraints: null },
        })
      })

      await waitFor(() => {
        const cached = queryClient.getQueryData<GameRoomSnapshotPayload>(
          queryKeys.games.snapshot(1)
        )
        expect(cached?.version).toBe(6)
        expect(cached?.snapshot.game.round_no).toBe(2)
        expect(cached?.viewerHand).toEqual(['AS'])
      })

      // (Optional) If your hook seeds version from initialData on rerender, this ensures no churn.
      rerender({ initialData: createInitialDataWithVersion(1, 2) })
      expect(mockWebSocketInstances.length).toBe(1)
    })

    it('triggers an HTTP refresh when receiving an error message', async () => {
      const initialData = createInitialDataWithVersion(1)

      renderHook(() => useGameSync({ initialData, gameId: 1 }), { queryClient })

      await waitForWsCount(1)
      const ws = mockWebSocketInstances[0]
      await connectAndSubscribe(ws, { expectedGameId: 1 })

      act(() => {
        serverSendJson(ws, {
          type: 'error',
          code: 'bad_request',
          message: 'Connection error',
        })
      })

      await waitFor(() => {
        expect(mockGetGameRoomSnapshotAction).toHaveBeenCalled()
      })
    })

    it('invalidates waitingLongest query when receiving long_wait_invalidated', async () => {
      const initialData = createInitialDataWithVersion(1)
      const invalidateSpy = vi.spyOn(queryClient, 'invalidateQueries')

      renderHook(() => useGameSync({ initialData, gameId: 1 }), { queryClient })

      await waitForWsCount(1)
      const ws = mockWebSocketInstances[0]
      await connectAndSubscribe(ws, { expectedGameId: 1 })

      act(() => {
        serverSendJson(ws, {
          type: 'long_wait_invalidated',
          game_id: 1,
        })
      })

      await waitFor(() => {
        expect(invalidateSpy).toHaveBeenCalledWith({
          queryKey: queryKeys.games.waitingLongest(),
        })
      })
    })
  })

  describe('Manual snapshot refresh', () => {
    it('refreshSnapshot calls action with etag and updates cache', async () => {
      const initialData = createInitialDataWithVersion(1, 1)

      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        { queryClient }
      )

      await waitForWsCount(1)
      const ws = mockWebSocketInstances[0]
      await connectAndSubscribe(ws, { expectedGameId: 1 })

      mockGetGameRoomSnapshotAction.mockResolvedValueOnce({
        kind: 'ok',
        data: createInitialDataWithVersion(1, 2),
      })

      await act(async () => {
        await result.current.refreshSnapshot()
      })

      expect(mockGetGameRoomSnapshotAction).toHaveBeenCalledWith({
        gameId: 1,
        etag: initialData.etag,
      })

      const cached = queryClient.getQueryData<GameRoomSnapshotPayload>(
        queryKeys.games.snapshot(1)
      )
      expect(cached?.version).toBe(2)
    })

    it('refreshSnapshot surfaces errors in syncError', async () => {
      const initialData = createInitialDataWithVersion(1)

      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        { queryClient }
      )

      await waitForWsCount(1)
      const ws = mockWebSocketInstances[0]
      await connectAndSubscribe(ws, { expectedGameId: 1 })

      mockGetGameRoomSnapshotAction.mockRejectedValueOnce(
        new Error('Network error')
      )

      await act(async () => {
        await result.current.refreshSnapshot()
      })

      expect(result.current.syncError).toBeDefined()
      expect(result.current.syncError?.message).toContain('Network error')
    })
  })
})
