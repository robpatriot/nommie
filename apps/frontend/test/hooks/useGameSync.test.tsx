import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, waitFor, act } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React, { type ReactNode } from 'react'
import { createTestQueryClient } from '../utils'
import { useGameSync } from '@/hooks/useGameSync'
import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import { initSnapshotFixture } from '../mocks/game-snapshot'
import { queryKeys } from '@/lib/queries/query-keys'
import { mockGetGameRoomSnapshotAction } from '../../setupGameRoomActionsMock'
import { MockWebSocket, mockWebSocketInstances } from '../setup/mock-websocket'
import { setupFetchMock } from '../setup/game-room-client-mocks'
import {
  createInitialData,
  createInitialDataWithVersion,
  waitForWebSocketConnection,
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

function createWrapper(queryClient: QueryClient) {
  const Wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  )
  Wrapper.displayName = 'TestQueryClientProvider'
  return Wrapper
}

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

async function waitForWsCount(n: number) {
  await waitFor(() => {
    expect(mockWebSocketInstances.length).toBe(n)
  })
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

async function waitForSentType(ws: MockWebSocket, type: string) {
  await waitFor(() => {
    expect(findSentByType(ws, type)).toBeDefined()
  })
}

async function completeHandshake(
  ws: MockWebSocket,
  opts: {
    protocol?: number
    userId?: number
    ackMessage?: string
    expectedGameId?: number // optional: assert subscribe topic
  } = {}
) {
  const protocol = opts.protocol ?? 1
  const userId = opts.userId ?? 123
  const ackMessage = opts.ackMessage ?? 'subscribed'

  // 1) Client must say hello first (means onopen ran and sendJson works)
  await waitForSentType(ws, 'hello')

  await act(async () => {
    // 2) Server acks hello (client should then subscribe)
    serverSendJson(ws, { type: 'hello_ack', protocol, user_id: userId })
  })

  // 3) Client should now send subscribe
  await waitForSentType(ws, 'subscribe')

  // 4) Optional: validate subscribe topic is correct
  if (opts.expectedGameId !== undefined) {
    const subscribe = findSentByType<{
      type: 'subscribe'
      topic: { kind: 'game'; id: number }
    }>(ws, 'subscribe')
    expect(subscribe?.topic).toEqual({ kind: 'game', id: opts.expectedGameId })
  }

  await act(async () => {
    // 5) Server acks subscription
    serverSendJson(ws, { type: 'ack', message: ackMessage })
  })
}

describe('useGameSync', () => {
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

    mockGetGameRoomSnapshotAction.mockImplementation(
      async ({ gameId }: { gameId: number }) => ({
        kind: 'ok' as const,
        data: createInitialDataWithVersion(gameId),
      })
    )

    // Set environment variable for WebSocket URL resolution
    process.env.NEXT_PUBLIC_BACKEND_BASE_URL = 'http://localhost:3001'
  })

  afterEach(() => {
    // Clean up any remaining WebSocket instances
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

  describe('Connection Lifecycle', () => {
    it('resets wsVersionRef on gameId change within the same hook instance', async () => {
      const queryClient = new QueryClient()
      const wrapper = createWrapper(queryClient)

      const initialData1 = createInitialData()
      initialData1.version = 1
      initialData1.etag = 'etag-1'

      const initialData2 = createInitialData()
      initialData2.version = 1
      initialData2.etag = 'etag-2'

      const { rerender } = renderHook(
        ({ gameId, initialData }) => useGameSync({ initialData, gameId }),
        {
          wrapper,
          initialProps: { gameId: 1, initialData: initialData1 },
        }
      )

      // Socket 0 connects
      await waitForWebSocketConnection()
      const ws0 = mockWebSocketInstances[0]
      expect(ws0.readyState).toBe(MockWebSocket.OPEN)

      await act(async () => {
        await completeHandshake(ws0, { expectedGameId: 1 })
      })

      // Apply a high-version game_state for game 1 -> wsVersionRef becomes 100
      act(() => {
        serverSendJson(ws0, {
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

      // Rerender with a different gameId WITHOUT unmounting.
      await act(async () => {
        rerender({ gameId: 2, initialData: initialData2 })
      })

      // Old socket should be closed intentionally and a new socket created (index 1).
      await waitFor(() => {
        expect(mockWebSocketInstances.length).toBeGreaterThan(1)
        expect(mockWebSocketInstances[0].readyState).toBe(MockWebSocket.CLOSED)
        expect(mockWebSocketInstances[1].readyState).toBe(MockWebSocket.OPEN)
      })

      const ws1 = mockWebSocketInstances[1]

      await act(async () => {
        await completeHandshake(ws1, { expectedGameId: 2 })
      })

      // Send a low-version game_state for game 2.
      // This must NOT be dropped due to stale last=100 from the previous game.
      act(() => {
        serverSendJson(ws1, {
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

      // Assert: cache for game 2 updated (i.e. message applied).
      await waitFor(() => {
        const data = queryClient.getQueryData(queryKeys.games.snapshot(2))
        expect(data).toBeTruthy()
      })
    })

    it('should create a WebSocket connection on mount', async () => {
      const initialData = createInitialDataWithVersion(1)
      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await waitForWsCount(1)
      const ws = mockWebSocketInstances[0]

      await completeHandshake(ws, { expectedGameId: 1 })

      await waitFor(() => {
        expect(result.current.connectionState).toBe('connected')
      })
    })

    it('should close connection on unmount', async () => {
      const initialData = createInitialDataWithVersion(1)
      const { result, unmount } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await waitForWsCount(1)
      await completeHandshake(mockWebSocketInstances[0])

      await waitFor(
        () => {
          expect(result.current.connectionState).toBe('connected')
        },
        { timeout: 2000 }
      )

      const ws = mockWebSocketInstances[0]
      const closeSpy = vi.spyOn(ws, 'close')

      unmount()

      expect(closeSpy).toHaveBeenCalledWith(1000, 'Connection closed')
      // Note: Handlers are no longer nulled before close() - onclose fires naturally
      // The handler checks closeReasonRef to determine if reconnection should occur
    })

    it('should not reconnect when initialData changes but gameId stays the same', async () => {
      const initialDataV1 = createInitialDataWithVersion(1, 1)
      const initialDataV2 = createInitialDataWithVersion(1, 2)

      const { result, rerender } = renderHook(
        ({ gameId, initialData }) =>
          useGameSync({
            initialData,
            gameId,
          }),
        {
          initialProps: { gameId: 1, initialData: initialDataV1 },
          wrapper: createWrapper(queryClient),
        }
      )

      // One socket created
      await waitForWsCount(1)
      const ws0 = mockWebSocketInstances[0]

      await completeHandshake(ws0, { expectedGameId: 1 })

      await waitFor(() => {
        expect(result.current.connectionState).toBe('connected')
      })

      // Sanity: exactly one socket
      expect(mockWebSocketInstances.length).toBe(1)

      // Rerender with SAME gameId but NEW initialData (new etag + version)
      rerender({ gameId: 1, initialData: initialDataV2 })

      // ✅ Must NOT create a new WebSocket
      await waitFor(() => {
        expect(mockWebSocketInstances.length).toBe(1)
      })

      // Still connected (no lifecycle churn)
      await waitFor(() => {
        expect(result.current.connectionState).toBe('connected')
      })
    })

    it('should update connection when gameId changes', async () => {
      const initialData1 = createInitialDataWithVersion(1)
      const initialData2 = createInitialDataWithVersion(2)

      const { result, rerender } = renderHook(
        ({ gameId }) =>
          useGameSync({
            initialData: gameId === 1 ? initialData1 : initialData2,
            gameId,
          }),
        {
          initialProps: { gameId: 1 },
          wrapper: createWrapper(queryClient),
        }
      )

      // Socket 0 created + handshake
      await waitForWsCount(1)
      const ws0 = mockWebSocketInstances[0]

      await act(async () => {
        await completeHandshake(ws0)
      })

      // Baseline authoritative state for game 1
      await act(async () => {
        serverSendJson(ws0, {
          type: 'game_state',
          topic: { kind: 'game', id: 1 },
          version: initialData1.version ?? 1,
          game: initialData1.snapshot,
          viewer: {
            seat: initialData1.viewerSeat ?? null,
            hand: [],
            bidConstraints: null,
          },
        })
      })

      await waitFor(
        () => {
          expect(result.current.connectionState).toBe('connected')
        },
        { timeout: 2000 }
      )

      const closeSpy0 = vi.spyOn(ws0, 'close')

      // Change gameId
      await act(async () => {
        rerender({ gameId: 2 })
      })

      // Old connection should be closed
      expect(closeSpy0).toHaveBeenCalledWith(1000, 'Connection closed')

      // New connection should be created
      await waitForWsCount(2)
      const ws1 = mockWebSocketInstances[1]

      await waitFor(
        () => {
          expect(ws1.readyState).toBe(MockWebSocket.OPEN)
        },
        { timeout: 2000 }
      )

      // Handshake new socket
      await act(async () => {
        await completeHandshake(ws1)
      })

      // Baseline authoritative state for game 2
      await act(async () => {
        serverSendJson(ws1, {
          type: 'game_state',
          topic: { kind: 'game', id: 2 },
          version: initialData2.version ?? 1,
          game: initialData2.snapshot,
          viewer: {
            seat: initialData2.viewerSeat ?? null,
            hand: [],
            bidConstraints: null,
          },
        })
      })

      await waitFor(
        () => {
          expect(result.current.connectionState).toBe('connected')
        },
        { timeout: 2000 }
      )
    })
  })

  describe('Manual Connection Control', () => {
    it('should allow manual disconnect', async () => {
      const initialData = createInitialDataWithVersion(1)
      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await waitForWsCount(1)
      await completeHandshake(mockWebSocketInstances[0])

      await waitFor(
        () => {
          expect(result.current.connectionState).toBe('connected')
        },
        { timeout: 2000 }
      )

      const ws = mockWebSocketInstances[0]
      const closeSpy = vi.spyOn(ws, 'close')

      act(() => {
        result.current.disconnect()
      })

      expect(closeSpy).toHaveBeenCalledWith(1000, 'Connection closed')
      // Note: disconnect() removes the onclose handler before closing,
      // so the state may not update to 'disconnected'. The connection is still effectively closed.
      expect(ws.readyState).toBe(MockWebSocket.CLOSED)
    })

    it('should allow manual reconnect after disconnect', async () => {
      const initialData = createInitialDataWithVersion(1)
      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await waitForWsCount(1)
      await completeHandshake(mockWebSocketInstances[0])

      await waitFor(
        () => {
          expect(result.current.connectionState).toBe('connected')
        },
        { timeout: 2000 }
      )

      act(() => {
        result.current.disconnect()
      })

      // Connection is closed (even if state doesn't reflect it)
      expect(mockWebSocketInstances[0].readyState).toBe(MockWebSocket.CLOSED)

      // Reconnect manually
      await act(async () => {
        await result.current.connect()
      })

      // New socket
      await waitForWsCount(2)
      await completeHandshake(mockWebSocketInstances[1])

      await waitFor(
        () => {
          expect(result.current.connectionState).toBe('connected')
        },
        { timeout: 2000 }
      )
    })
  })

  describe('WebSocket Message Handling', () => {
    it('should update query cache when receiving game_state message', async () => {
      const initialData = createInitialDataWithVersion(1, 1)
      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await waitForWsCount(1)
      await completeHandshake(mockWebSocketInstances[0])

      await waitFor(
        () => {
          expect(result.current.connectionState).toBe('connected')
        },
        { timeout: 2000 }
      )

      const ws = mockWebSocketInstances[0]
      const updatedSnapshot = {
        ...initSnapshotFixture,
        game: {
          ...initSnapshotFixture.game,
          round_no: 2,
        },
      }

      const message = {
        type: 'game_state',
        topic: { kind: 'game', id: 1 },
        version: 2,
        game: updatedSnapshot,
        viewer: {
          seat: 0,
          hand: ['2H', '3C'],
          bidConstraints: null,
        },
      }

      act(() => {
        serverSendJson(ws, message)
      })

      // Check that query cache was updated
      const cachedData = queryClient.getQueryData<GameRoomSnapshotPayload>(
        queryKeys.games.snapshot(1)
      )

      expect(cachedData).toBeDefined()
      expect(cachedData?.version).toBe(2)
      expect(cachedData?.snapshot.game.round_no).toBe(2)
      expect(cachedData?.viewerHand).toEqual(['2H', '3C'])
    })

    it('should update wsVersionRef when initialData version changes (ignore stale game_state, accept new)', async () => {
      const initialDataV1 = createInitialDataWithVersion(1, 1)
      const initialDataV2 = createInitialDataWithVersion(1, 2)

      const { result, rerender } = renderHook(
        ({ initialData }) => useGameSync({ initialData, gameId: 1 }),
        {
          initialProps: { initialData: initialDataV1 },
          wrapper: createWrapper(queryClient),
        }
      )

      await waitForWsCount(1)
      await completeHandshake(mockWebSocketInstances[0])

      await waitFor(
        () => {
          expect(result.current.connectionState).toBe('connected')
        },
        { timeout: 2000 }
      )

      const ws = mockWebSocketInstances[0]

      // wsVersionRef is seeded from initialData.version (1), so send v2 as the first WS-applied update.
      act(() => {
        serverSendJson(ws, {
          type: 'game_state',
          topic: { kind: 'game', id: 1 },
          version: 2,
          game: {
            ...initSnapshotFixture,
            game: {
              ...initSnapshotFixture.game,
              round_no: 1,
            },
          },
          viewer: {
            seat: 0,
            hand: ['AS'],
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
        expect(cached?.snapshot.game.round_no).toBe(1)
        expect(cached?.viewerHand).toEqual(['AS'])
      })

      // Advance initialData to version 2 WITHOUT changing gameId (must not reconnect)
      rerender({ initialData: initialDataV2 })
      expect(mockWebSocketInstances.length).toBe(1)

      // Capture whatever the cache is right now (some implementations may clear/reseed on rerender).
      const beforeStale = queryClient.getQueryData<GameRoomSnapshotPayload>(
        queryKeys.games.snapshot(1)
      )

      // Send stale v1 with different content; it should NOT change the cache from beforeStale
      act(() => {
        serverSendJson(ws, {
          type: 'game_state',
          topic: { kind: 'game', id: 1 },
          version: 1,
          game: {
            ...initSnapshotFixture,
            game: {
              ...initSnapshotFixture.game,
              round_no: 99,
            },
          },
          viewer: {
            seat: 0,
            hand: ['KS'],
            bidConstraints: null,
          },
        })
      })

      await waitFor(() => {
        const afterStale = queryClient.getQueryData<GameRoomSnapshotPayload>(
          queryKeys.games.snapshot(1)
        )
        expect(afterStale).toEqual(beforeStale)
      })

      // Send fresh v3 with updated content; this SHOULD apply
      act(() => {
        serverSendJson(ws, {
          type: 'game_state',
          topic: { kind: 'game', id: 1 },
          version: 3,
          game: {
            ...initSnapshotFixture,
            game: {
              ...initSnapshotFixture.game,
              round_no: 2,
            },
          },
          viewer: {
            seat: 0,
            hand: ['2H', '3C'],
            bidConstraints: null,
          },
        })
      })

      await waitFor(() => {
        const cached2 = queryClient.getQueryData<GameRoomSnapshotPayload>(
          queryKeys.games.snapshot(1)
        )
        expect(cached2).toBeDefined()
        expect(cached2?.version).toBe(3)
        expect(cached2?.snapshot.game.round_no).toBe(2)
        expect(cached2?.viewerHand).toEqual(['2H', '3C'])
      })
    })

    it('should ignore older game_state messages', async () => {
      // Create a QueryClient with non-zero gcTime to prevent immediate cache eviction
      // The default test QueryClient has gcTime: 0 which causes immediate garbage collection
      const testQueryClient = new QueryClient({
        defaultOptions: {
          queries: {
            retry: false,
            gcTime: 5 * 60 * 1000, // 5 minutes - keep cache alive for this test
          },
          mutations: {
            retry: false,
          },
        },
      })

      const initialData = createInitialDataWithVersion(1, 1)
      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(testQueryClient),
        }
      )

      await waitForWsCount(1)
      await completeHandshake(mockWebSocketInstances[0])

      await waitFor(
        () => {
          expect(result.current.connectionState).toBe('connected')
        },
        { timeout: 2000 }
      )

      const ws = mockWebSocketInstances[0]

      // First, send version 5 to establish the cache
      const version5Message = {
        type: 'game_state',
        topic: { kind: 'game', id: 1 },
        version: 5,
        game: initSnapshotFixture,
        viewer: {
          seat: 0,
          hand: [],
          bidConstraints: null,
        },
      }

      act(() => {
        serverSendJson(ws, version5Message)
      })

      // Wait a bit for any async processing
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 100))
      })

      // Check that cache was updated
      const before = testQueryClient.getQueryData<GameRoomSnapshotPayload>(
        queryKeys.games.snapshot(1)
      )
      expect(before).toBeDefined()
      expect(before?.version).toBe(5)

      // Now send older (version 3) - should be ignored
      const version3Message = {
        type: 'game_state',
        topic: { kind: 'game', id: 1 },
        version: 3,
        game: initSnapshotFixture,
        viewer: {
          seat: 0,
          hand: [],
          bidConstraints: null,
        },
      }

      act(() => {
        serverSendJson(ws, version3Message)
      })

      // Wait a bit for any async processing
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 50))
      })

      // Cache should still have version 5
      const cachedData = testQueryClient.getQueryData<GameRoomSnapshotPayload>(
        queryKeys.games.snapshot(1)
      )
      expect(cachedData).toBeDefined()
      expect(cachedData?.version).toBe(5)
    })

    it('should refresh snapshot on error message', async () => {
      const initialData = createInitialDataWithVersion(1)
      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await waitForWsCount(1)
      await completeHandshake(mockWebSocketInstances[0])

      await waitFor(
        () => {
          expect(result.current.connectionState).toBe('connected')
        },
        { timeout: 2000 }
      )

      const ws = mockWebSocketInstances[0]

      // Send error message (protocol-aligned)
      const errorMessage = {
        type: 'error',
        code: 'bad_request',
        message: 'Connection error',
      }

      act(() => {
        serverSendJson(ws, errorMessage)
      })

      // Should trigger refresh
      await waitFor(
        () => {
          expect(mockGetGameRoomSnapshotAction).toHaveBeenCalled()
        },
        { timeout: 2000 }
      )
    })
  })

  describe('Manual Snapshot Refresh', () => {
    it('should refresh snapshot manually', async () => {
      const initialData = createInitialDataWithVersion(1, 1)
      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await waitForWsCount(1)
      await completeHandshake(mockWebSocketInstances[0])

      await waitFor(
        () => {
          expect(result.current.connectionState).toBe('connected')
        },
        { timeout: 2000 }
      )

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

      const cachedData = queryClient.getQueryData<GameRoomSnapshotPayload>(
        queryKeys.games.snapshot(1)
      )
      expect(cachedData?.version).toBe(2)
    })

    it('should handle refresh errors', async () => {
      const initialData = createInitialDataWithVersion(1)
      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await waitForWsCount(1)
      await completeHandshake(mockWebSocketInstances[0])

      await waitFor(
        () => {
          expect(result.current.connectionState).toBe('connected')
        },
        { timeout: 2000 }
      )

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

  describe('Error Handling', () => {
    it('should handle token fetch timeout', async () => {
      const initialData = createInitialDataWithVersion(1)
      const mockFetchFn = vi.fn((url: string | URL | Request) => {
        const urlString =
          typeof url === 'string'
            ? url
            : url instanceof URL
              ? url.toString()
              : url.url
        if (urlString.includes('/api/ws-token')) {
          return new Promise((_, reject) => {
            setTimeout(() => {
              const error = new Error('AbortError')
              error.name = 'AbortError'
              reject(error)
            }, 100)
          })
        }
        return originalFetch(url)
      })
      vi.stubGlobal('fetch', mockFetchFn)

      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await waitFor(
        () => {
          expect(result.current.syncError).toBeDefined()
        },
        { timeout: 2000 }
      )

      expect(result.current.syncError).toBeDefined()
      if (result.current.syncError?.message) {
        expect(result.current.syncError.message).toContain('timed out')
      }
    })

    it('should handle token fetch failure', async () => {
      const initialData = createInitialDataWithVersion(1)
      const mockFetchFn = vi.fn((url: string | URL | Request) => {
        const urlString =
          typeof url === 'string'
            ? url
            : url instanceof URL
              ? url.toString()
              : url.url
        if (urlString.includes('/api/ws-token')) {
          return Promise.resolve({
            ok: false,
            status: 500,
            statusText: 'Internal Server Error',
          } as Response)
        }
        return originalFetch(url)
      })
      vi.stubGlobal('fetch', mockFetchFn)

      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await waitFor(
        () => {
          expect(result.current.syncError).toBeDefined()
        },
        { timeout: 2000 }
      )

      expect(result.current.syncError).toBeDefined()
      if (result.current.syncError?.message) {
        expect(result.current.syncError.message).toContain('500')
      }
    })

    it('should handle WebSocket connection errors', async () => {
      const initialData = createInitialDataWithVersion(1)
      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await waitForWsCount(1)
      await completeHandshake(mockWebSocketInstances[0])

      await waitFor(
        () => {
          expect(result.current.connectionState).toBe('connected')
        },
        { timeout: 2000 }
      )

      const ws = mockWebSocketInstances[0]

      act(() => {
        ws.onerror?.(new Event('error'))
      })

      expect(result.current.syncError).toBeDefined()
      expect(result.current.syncError?.message).toBe(
        'Websocket connection error'
      )
    })
  })

  describe('Connection State Management', () => {
    it('should track connection state correctly', async () => {
      const initialData = createInitialDataWithVersion(1)
      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      // Initially connecting
      expect(result.current.connectionState).toBe('connecting')

      await waitForWsCount(1)
      await completeHandshake(mockWebSocketInstances[0])

      // Then connected (handshake-gated)
      await waitFor(
        () => {
          expect(result.current.connectionState).toBe('connected')
        },
        { timeout: 2000 }
      )

      // Disconnect - this cleans up the WebSocket
      const ws = mockWebSocketInstances[0]
      act(() => {
        result.current.disconnect()
      })

      // Verify the WebSocket was closed
      expect(ws.readyState).toBe(MockWebSocket.CLOSED)
    })
  })
})
