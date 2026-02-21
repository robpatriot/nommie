// ============================================================================
// FILE: apps/frontend/test/hooks/useGameSync.test.tsx
// ============================================================================

import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, waitFor, act, createTestQueryClient } from '../utils'
import type { QueryClient } from '@tanstack/react-query'
import { useGameSync } from '@/hooks/useGameSync'
import { initSnapshotFixture } from '../mocks/game-snapshot'
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
  createInitialState,
  createInitialStateWithVersion,
  createStateWithVersionForMock,
} from '../setup/game-room-client-helpers'
import { TEST_BACKEND_WS_URL } from '@/test/setup/test-constants'

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
  resolveWebSocketUrl: () => TEST_BACKEND_WS_URL,
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
    serverSendJson(ws, {
      type: 'ack',
      command: 'subscribe',
      topic: { kind: 'game', id: opts.expectedGameId },
    })
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

    mocks.getWaitingLongestGameAction.mockResolvedValue({
      kind: 'ok',
      data: [],
    })

    mockGetGameRoomStateAction.mockImplementation(
      async ({ gameId }: { gameId: number }) => ({
        kind: 'ok' as const,
        data: createStateWithVersionForMock(gameId),
      })
    )
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
      const initialState = createInitialStateWithVersion(1)

      const { result } = renderHook(
        () => useGameSync({ initialState, gameId: 1 }),
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
      const initialState1 = createInitialState(1, initSnapshotFixture, {
        version: 1,
        etag: 'etag-1',
      })
      const initialState2 = createInitialState(2, initSnapshotFixture, {
        version: 1,
        etag: 'etag-2',
      })

      const { rerender } = renderHook(
        ({ gameId, initialState }) => useGameSync({ initialState, gameId }),
        {
          queryClient,
          initialProps: { gameId: 1, initialState: initialState1 },
        }
      )

      await waitForWsCount(1)
      const ws = mockWebSocketInstances[0]
      await connectAndSubscribe(ws, { expectedGameId: 1 })

      act(() => {
        serverSendJson(ws, {
          type: 'game_state',
          topic: { kind: 'game', id: 1 },
          version: 100,
          game: initialState1.game,
          viewer: {
            seat: initialState1.viewer.seat ?? null,
            hand: [],
            bidConstraints: null,
          },
        })
      })

      await act(async () => {
        rerender({ gameId: 2, initialState: initialState2 })
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
        serverSendJson(ws, {
          type: 'ack',
          command: 'subscribe',
          topic: { kind: 'game', id: 2 },
        })
      })

      act(() => {
        serverSendJson(ws, {
          type: 'game_state',
          topic: { kind: 'game', id: 2 },
          version: 2,
          game: initialState2.game,
          viewer: {
            seat: initialState2.viewer.seat ?? null,
            hand: [],
            bidConstraints: null,
          },
        })
      })

      await waitFor(() => {
        const data = queryClient.getQueryData(queryKeys.games.state(2))
        expect(data).toBeTruthy()
      })
    })

    it('does not re-subscribe when initialState changes but gameId stays the same', async () => {
      const initialStateV1 = createInitialStateWithVersion(1, 1)
      const initialStateV2 = createInitialStateWithVersion(1, 2)

      const { rerender } = renderHook(
        ({ gameId, initialState }) => useGameSync({ initialState, gameId }),
        {
          queryClient,
          initialProps: { gameId: 1, initialState: initialStateV1 },
        }
      )

      await waitForWsCount(1)
      const ws = mockWebSocketInstances[0]
      await connectAndSubscribe(ws, { expectedGameId: 1 })

      const subscribeCountBefore = getSentJson(ws).filter(
        (m) =>
          typeof m === 'object' && m !== null && (m as any).type === 'subscribe'
      ).length

      rerender({ gameId: 1, initialState: initialStateV2 })

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
      const initialState = createInitialStateWithVersion(1, 1)

      renderHook(() => useGameSync({ initialState, gameId: 1 }), {
        queryClient,
      })

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
        const cached = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(1)
        )
        expect(cached).toBeDefined()
        expect(selectVersion(cached!)).toBe(2)
        expect(selectSnapshot(cached!).game.round_no).toBe(2)
        expect(selectViewerHand(cached!)).toEqual(['2H', '3C'])
      })
    })

    it('ignores stale game_state and applies newer ones', async () => {
      const initialStateV1 = createInitialStateWithVersion(1, 1)

      const { rerender } = renderHook(
        ({ initialState }) => useGameSync({ initialState, gameId: 1 }),
        {
          queryClient,
          initialProps: { initialState: initialStateV1 },
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
        const cached = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(1)
        )
        expect(cached).toBeDefined()
        expect(selectVersion(cached!)).toBe(5)
      })

      const before = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(1)
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
        const afterStale = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(1)
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
        const cached = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(1)
        )
        expect(cached).toBeDefined()
        expect(selectVersion(cached!)).toBe(6)
        expect(selectSnapshot(cached!).game.round_no).toBe(2)
        expect(selectViewerHand(cached!)).toEqual(['AS'])
      })

      // (Optional) If your hook seeds version from initialState on rerender, this ensures no churn.
      rerender({ initialState: createInitialStateWithVersion(1, 2) })
      expect(mockWebSocketInstances.length).toBe(1)
    })

    it('triggers an HTTP refresh when receiving an error message', async () => {
      const initialState = createInitialStateWithVersion(1)

      renderHook(() => useGameSync({ initialState, gameId: 1 }), {
        queryClient,
      })

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
        expect(mockGetGameRoomStateAction).toHaveBeenCalled()
      })
    })

    it('refetches LW cache when receiving long_wait_invalidated', async () => {
      const initialState = createInitialStateWithVersion(1)

      renderHook(() => useGameSync({ initialState, gameId: 1 }), {
        queryClient,
      })

      await waitForWsCount(1)
      const ws = mockWebSocketInstances[0]
      await connectAndSubscribe(ws, { expectedGameId: 1 })

      mocks.getWaitingLongestGameAction.mockResolvedValue({
        kind: 'ok',
        data: [1],
      })
      mocks.getWaitingLongestGameAction.mockClear()

      act(() => {
        serverSendJson(ws, {
          type: 'long_wait_invalidated',
          game_id: 1,
        })
      })

      await waitFor(() => {
        expect(mocks.getWaitingLongestGameAction).toHaveBeenCalledTimes(1)
      })
    })
  })

  describe('Manual snapshot refresh', () => {
    it('refreshStateFromHttp calls action with etag and updates cache', async () => {
      const initialState = createInitialStateWithVersion(1, 1)

      const { result } = renderHook(
        () => useGameSync({ initialState, gameId: 1 }),
        { queryClient }
      )

      await waitForWsCount(1)
      const ws = mockWebSocketInstances[0]
      await connectAndSubscribe(ws, { expectedGameId: 1 })

      mockGetGameRoomStateAction.mockResolvedValueOnce({
        kind: 'ok',
        data: createStateWithVersionForMock(1, 2),
      })

      await act(async () => {
        await result.current.refreshStateFromHttp()
      })

      expect(mockGetGameRoomStateAction).toHaveBeenCalledWith({
        gameId: 1,
        etag: initialState.etag,
      })

      const cached = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(1)
      )
      expect(cached).toBeDefined()
      expect(selectVersion(cached!)).toBe(2)
    })

    it('refreshStateFromHttp does not update cache on not_modified (avoids receivedAt-only churn)', async () => {
      const initialState = createInitialStateWithVersion(1, 1)

      const { result } = renderHook(
        () => useGameSync({ initialState, gameId: 1 }),
        { queryClient }
      )

      await waitForWsCount(1)
      const ws = mockWebSocketInstances[0]
      await connectAndSubscribe(ws, { expectedGameId: 1 })

      act(() => {
        serverSendJson(ws, {
          type: 'game_state',
          topic: { kind: 'game', id: 1 },
          version: 2,
          game: initSnapshotFixture,
          viewer: { seat: 0, hand: [], bidConstraints: null },
        })
      })

      await waitFor(() => {
        const cached = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(1)
        )
        expect(cached).toBeDefined()
        expect(selectVersion(cached!)).toBe(2)
      })

      const cachedBeforeRefresh = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(1)
      )
      mockGetGameRoomStateAction.mockResolvedValueOnce({
        kind: 'not_modified',
      })

      await act(async () => {
        await result.current.refreshStateFromHttp()
      })

      const cachedAfterRefresh = queryClient.getQueryData<GameRoomState>(
        queryKeys.games.state(1)
      )
      expect(cachedAfterRefresh).toBe(cachedBeforeRefresh)
    })

    it('does not synthesize ETag from WS payload version; ETag stays from initialState', async () => {
      const initialState = createInitialStateWithVersion(1, 1)
      const httpEtag = initialState.etag
      expect(httpEtag).toBe('"game-1-v1"')

      const { result } = renderHook(
        () => useGameSync({ initialState, gameId: 1 }),
        { queryClient }
      )

      await waitForWsCount(1)
      const ws = mockWebSocketInstances[0]
      await connectAndSubscribe(ws, { expectedGameId: 1 })

      mockGetGameRoomStateAction.mockClear()

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
        const cached = queryClient.getQueryData<GameRoomState>(
          queryKeys.games.state(1)
        )
        expect(cached).toBeDefined()
        expect(selectVersion(cached!)).toBe(5)
      })

      await act(async () => {
        await result.current.refreshStateFromHttp()
      })

      expect(mockGetGameRoomStateAction).toHaveBeenCalledWith({
        gameId: 1,
        etag: httpEtag,
      })
    })

    it('refreshStateFromHttp surfaces errors in syncError', async () => {
      const initialState = createInitialStateWithVersion(1)

      const { result } = renderHook(
        () => useGameSync({ initialState, gameId: 1 }),
        { queryClient }
      )

      await waitForWsCount(1)
      const ws = mockWebSocketInstances[0]
      await connectAndSubscribe(ws, { expectedGameId: 1 })

      mockGetGameRoomStateAction.mockRejectedValueOnce(
        new Error('Network error')
      )

      await act(async () => {
        await result.current.refreshStateFromHttp()
      })

      expect(result.current.syncError).toBeDefined()
      expect(result.current.syncError?.message).toContain('Network error')
    })
  })
})
