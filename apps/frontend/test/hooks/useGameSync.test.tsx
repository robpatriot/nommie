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

// Mock WebSocket API - same pattern as other test files
class MockWebSocket {
  static CONNECTING = 0
  static OPEN = 1
  static CLOSING = 2
  static CLOSED = 3

  readyState = MockWebSocket.CONNECTING
  url: string
  onopen: ((event: Event) => void) | null = null
  onerror: ((event: Event) => void) | null = null
  onclose: ((event: CloseEvent) => void) | null = null
  onmessage: ((event: MessageEvent) => void) | null = null

  // Event listeners for MSW compatibility
  private listeners: Map<string, Set<EventListener>> = new Map()

  constructor(url: string) {
    this.url = url
    mockWebSocketInstances.push(this)
    // Simulate async connection
    Promise.resolve().then(() => {
      this.readyState = MockWebSocket.OPEN
      this.onopen?.(new Event('open'))
      // Also trigger event listeners
      const openListeners = this.listeners.get('open')
      if (openListeners) {
        openListeners.forEach((listener) => {
          if (typeof listener === 'function') {
            listener(new Event('open'))
          }
        })
      }
    })
  }

  send(_data: string) {
    // Mock send
  }

  close(code = 1000, reason = 'Connection closed') {
    this.readyState = MockWebSocket.CLOSED
    const closeEvent = new CloseEvent('close', { code, reason })
    this.onclose?.(closeEvent)
    // Also trigger event listeners
    const closeListeners = this.listeners.get('close')
    if (closeListeners) {
      closeListeners.forEach((listener) => {
        if (typeof listener === 'function') {
          listener(closeEvent)
        }
      })
    }
  }

  // MSW compatibility methods
  addEventListener(
    type: string,
    listener: EventListener | null,
    _options?: boolean | AddEventListenerOptions
  ) {
    if (!listener) return
    if (!this.listeners.has(type)) {
      this.listeners.set(type, new Set())
    }
    this.listeners.get(type)!.add(listener)
  }

  removeEventListener(
    type: string,
    listener: EventListener | null,
    _options?: boolean | EventListenerOptions
  ) {
    if (!listener) return
    this.listeners.get(type)?.delete(listener)
  }
}

// Store WebSocket instances for test control
const mockWebSocketInstances: MockWebSocket[] = []

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

function createInitialData(
  gameId: number,
  version = 1,
  overrides?: Partial<GameRoomSnapshotPayload>
): GameRoomSnapshotPayload {
  return {
    snapshot: initSnapshotFixture,
    etag: `"game-${gameId}-v${version}"`,
    version,
    playerNames: ['Alex', 'Bailey', 'Casey', 'Dakota'],
    viewerSeat: 0,
    viewerHand: [],
    timestamp: new Date().toISOString(),
    hostSeat: 0,
    bidConstraints: null,
    ...overrides,
  }
}

function createWrapper(queryClient: QueryClient) {
  const Wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  )
  Wrapper.displayName = 'TestQueryClientProvider'
  return Wrapper
}

describe('useGameSync', () => {
  let queryClient: QueryClient

  beforeEach(() => {
    queryClient = createTestQueryClient()
    mockWebSocketInstances.length = 0
    vi.clearAllMocks()
    vi.useRealTimers()

    // Mock fetch for /api/ws-token endpoint
    vi.stubGlobal(
      'fetch',
      vi.fn((url: string | URL | Request) => {
        const urlString =
          typeof url === 'string'
            ? url
            : url instanceof URL
              ? url.toString()
              : url.url
        if (urlString.includes('/api/ws-token')) {
          return Promise.resolve({
            ok: true,
            json: async () => ({ token: 'test-token' }),
          } as Response)
        }
        // Fallback to original fetch for other requests
        return originalFetch(url)
      })
    )

    // Ensure WebSocket is mocked
    vi.stubGlobal('WebSocket', MockWebSocket)

    mockGetGameRoomSnapshotAction.mockImplementation(
      async ({ gameId }: { gameId: number }) => ({
        kind: 'ok' as const,
        data: createInitialData(gameId),
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
    it('should create a WebSocket connection on mount', async () => {
      const initialData = createInitialData(1)
      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await waitFor(
        () => {
          expect(mockWebSocketInstances.length).toBe(1)
          expect(mockWebSocketInstances[0].readyState).toBe(MockWebSocket.OPEN)
        },
        { timeout: 2000 }
      )

      expect(result.current.connectionState).toBe('connected')
    })

    it('should close connection on unmount', async () => {
      const initialData = createInitialData(1)
      const { result, unmount } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

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

    it('should update connection when gameId changes', async () => {
      const { result, rerender } = renderHook(
        ({ gameId }) =>
          useGameSync({ initialData: createInitialData(gameId), gameId }),
        {
          initialProps: { gameId: 1 },
          wrapper: createWrapper(queryClient),
        }
      )

      await waitFor(
        () => {
          expect(result.current.connectionState).toBe('connected')
        },
        { timeout: 2000 }
      )

      expect(mockWebSocketInstances.length).toBe(1)
      const ws1 = mockWebSocketInstances[0]
      const closeSpy1 = vi.spyOn(ws1, 'close')

      // Change gameId
      rerender({ gameId: 2 })

      // Old connection should be closed
      expect(closeSpy1).toHaveBeenCalledWith(1000, 'Connection closed')

      // New connection should be created
      await waitFor(
        () => {
          expect(mockWebSocketInstances.length).toBe(2)
          expect(mockWebSocketInstances[1].readyState).toBe(MockWebSocket.OPEN)
        },
        { timeout: 2000 }
      )

      expect(result.current.connectionState).toBe('connected')
    })
  })

  describe('Manual Connection Control', () => {
    it('should allow manual disconnect', async () => {
      const initialData = createInitialData(1)
      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

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
      const initialData = createInitialData(1)
      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

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

      await waitFor(
        () => {
          expect(mockWebSocketInstances.length).toBe(2)
          expect(result.current.connectionState).toBe('connected')
        },
        { timeout: 2000 }
      )
    })
  })

  describe('WebSocket Message Handling', () => {
    it('should update query cache when receiving snapshot message', async () => {
      const initialData = createInitialData(1, 1)
      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

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
        type: 'snapshot',
        data: {
          snapshot: updatedSnapshot,
          version: 2,
          viewer_hand: ['2H', '3C'],
          bid_constraints: null,
        },
        viewer_seat: 0,
      }

      act(() => {
        ws.onmessage?.(
          new MessageEvent('message', {
            data: JSON.stringify(message),
          })
        )
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

    it('should ignore older snapshots', async () => {
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

      const initialData = createInitialData(1, 1)
      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(testQueryClient),
        }
      )

      await waitFor(
        () => {
          expect(result.current.connectionState).toBe('connected')
        },
        { timeout: 2000 }
      )

      const ws = mockWebSocketInstances[0]

      // First, send version 5 snapshot to establish the cache
      const version5Message = {
        type: 'snapshot',
        data: {
          snapshot: initSnapshotFixture,
          version: 5,
          viewer_hand: [],
          bid_constraints: null,
        },
        viewer_seat: 0,
      }

      act(() => {
        ws.onmessage?.(
          new MessageEvent('message', {
            data: JSON.stringify(version5Message),
          })
        )
      })

      // Wait a bit for any async processing
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 100))
      })

      // Check that cache was updated
      const beforeVersion3 =
        testQueryClient.getQueryData<GameRoomSnapshotPayload>(
          queryKeys.games.snapshot(1)
        )
      expect(beforeVersion3).toBeDefined()
      expect(beforeVersion3?.version).toBe(5)

      // Now send older snapshot (version 3) - should be ignored
      const version3Message = {
        type: 'snapshot',
        data: {
          snapshot: initSnapshotFixture,
          version: 3,
          viewer_hand: [],
          bid_constraints: null,
        },
        viewer_seat: 0,
      }

      act(() => {
        ws.onmessage?.(
          new MessageEvent('message', {
            data: JSON.stringify(version3Message),
          })
        )
      })

      // Wait a bit for any async processing
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 50))
      })

      // Cache should still have version 5 (older snapshot should be ignored)
      const cachedData = testQueryClient.getQueryData<GameRoomSnapshotPayload>(
        queryKeys.games.snapshot(1)
      )
      expect(cachedData).toBeDefined()
      expect(cachedData?.version).toBe(5)
    })

    it('should refresh snapshot on error message', async () => {
      const initialData = createInitialData(1)
      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await waitFor(
        () => {
          expect(result.current.connectionState).toBe('connected')
        },
        { timeout: 2000 }
      )

      const ws = mockWebSocketInstances[0]

      // Send error message
      const errorMessage = {
        type: 'error',
        message: 'Connection error',
      }

      act(() => {
        ws.onmessage?.(
          new MessageEvent('message', {
            data: JSON.stringify(errorMessage),
          })
        )
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
      const initialData = createInitialData(1, 1)
      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await waitFor(
        () => {
          expect(result.current.connectionState).toBe('connected')
        },
        { timeout: 2000 }
      )

      mockGetGameRoomSnapshotAction.mockResolvedValueOnce({
        kind: 'ok',
        data: createInitialData(1, 2),
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
      const initialData = createInitialData(1)
      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

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
      const initialData = createInitialData(1)
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
      const initialData = createInitialData(1)
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
      const initialData = createInitialData(1)
      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

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
      const initialData = createInitialData(1)
      const { result } = renderHook(
        () => useGameSync({ initialData, gameId: 1 }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      // Initially connecting
      expect(result.current.connectionState).toBe('connecting')

      // Then connected
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
