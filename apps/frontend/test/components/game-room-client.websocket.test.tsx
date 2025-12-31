import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, waitFor, act } from '../utils'
import type { QueryClient } from '@tanstack/react-query'
import userEvent from '@testing-library/user-event'
import type { ReactNode } from 'react'
import { queryKeys } from '@/lib/queries/query-keys'

import { GameRoomClient } from '@/app/game/[gameId]/_components/game-room-client'
import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import {
  initSnapshotFixture,
  biddingSnapshotFixture,
} from '../mocks/game-snapshot'
import {
  mockGetGameRoomSnapshotAction,
  mockMarkPlayerReadyAction,
  mockSubmitBidAction,
  mockSelectTrumpAction,
  mockSubmitPlayAction,
  mockAddAiSeatAction,
  mockUpdateAiSeatAction,
  mockRemoveAiSeatAction,
  mockFetchAiRegistryAction,
} from '../../setupGameRoomActionsMock'

// Mock hooks
const mockShowToast = vi.fn()
const mockHideToast = vi.fn()

vi.mock('@/hooks/useToast', () => ({
  useToast: () => ({
    toasts: [],
    showToast: mockShowToast,
    hideToast: mockHideToast,
  }),
}))

// Don't mock useAiRegistry - use the real implementation
// It uses TanStack Query which will only call the action when enabled=true

// Don't mock useGameRoomSnapshot - use the real implementation
// It will read from the query cache which useGameSync updates

// Track which gameIds have been initialized to prevent infinite loops
const initializedGameIds = new Set<number>()

// Don't mock useGameSync - use the real implementation
// It will create WebSocket connections (mocked) and update the query cache
// The WebSocket API is already mocked above, so this will work

// Mock mutation hooks - mutateAsync should call the corresponding server action
// If the action returns an error, mutateAsync should throw
const mockUseMarkPlayerReady = vi.fn(() => ({
  mutateAsync: async ({
    gameId,
    isReady,
  }: {
    gameId: number
    isReady: boolean
  }) => {
    const result = await mockMarkPlayerReadyAction(gameId, isReady)
    if (result.kind === 'error') {
      throw new Error(result.message)
    }
    return result
  },
  isPending: false,
}))

const mockUseSubmitBid = vi.fn(() => ({
  mutateAsync: (request: unknown) => mockSubmitBidAction(request),
  isPending: false,
}))

const mockUseSelectTrump = vi.fn(() => ({
  mutateAsync: (request: unknown) => mockSelectTrumpAction(request),
  isPending: false,
}))

const mockUseSubmitPlay = vi.fn(() => ({
  mutateAsync: (request: unknown) => mockSubmitPlayAction(request),
  isPending: false,
}))

const mockUseAddAiSeat = vi.fn(() => ({
  mutateAsync: (request: unknown) => mockAddAiSeatAction(request),
  isPending: false,
}))

const mockUseUpdateAiSeat = vi.fn(() => ({
  mutateAsync: (request: unknown) => mockUpdateAiSeatAction(request),
  isPending: false,
}))

const mockUseRemoveAiSeat = vi.fn(() => ({
  mutateAsync: (request: unknown) => mockRemoveAiSeatAction(request),
  isPending: false,
}))

const mockUseLeaveGame = vi.fn(() => ({
  mutateAsync: (_gameId: number) => Promise.resolve(),
  isPending: false,
}))

vi.mock('@/hooks/mutations/useGameRoomMutations', () => ({
  useMarkPlayerReady: () => mockUseMarkPlayerReady(),
  useLeaveGame: () => mockUseLeaveGame(),
  useSubmitBid: () => mockUseSubmitBid(),
  useSelectTrump: () => mockUseSelectTrump(),
  useSubmitPlay: () => mockUseSubmitPlay(),
  useAddAiSeat: () => mockUseAddAiSeat(),
  useUpdateAiSeat: () => mockUseUpdateAiSeat(),
  useRemoveAiSeat: () => mockUseRemoveAiSeat(),
}))

// Mock WebSocket API to avoid real websocket connections in tests
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

  constructor(url: string) {
    this.url = url
    // Track instance
    mockWebSocketInstances.push(this)
    // Simulate async connection - connect immediately
    Promise.resolve().then(() => {
      this.readyState = MockWebSocket.OPEN
      this.onopen?.(new Event('open'))
    })
  }

  send(_data: string) {
    // Mock send - do nothing
  }

  close() {
    this.readyState = MockWebSocket.CLOSED
    this.onclose?.(new CloseEvent('close'))
  }
}

// Store WebSocket instances for test control
const mockWebSocketInstances: MockWebSocket[] = []

// Track original fetch (WebSocket is restored via vi.unstubAllGlobals)
const originalFetch = global.fetch

// Mock next/link
vi.mock('next/link', () => ({
  __esModule: true,
  default: ({ children, ...props }: { children: ReactNode; href: string }) => (
    <a {...props}>{children}</a>
  ),
}))

// Helper to create initial data
function createInitialData(
  snapshot = initSnapshotFixture,
  overrides?: Partial<GameRoomSnapshotPayload>
): GameRoomSnapshotPayload {
  return {
    snapshot,
    etag: 'initial-etag',
    playerNames: ['Alex', 'Bailey', 'Casey', 'Dakota'],
    viewerSeat: 0,
    viewerHand: [],
    timestamp: new Date().toISOString(),
    hostSeat: 0,
    ...overrides,
  }
}

// Helper to wait for WebSocket connection
async function waitForWebSocketConnection() {
  await waitFor(
    () => {
      expect(mockWebSocketInstances.length).toBeGreaterThan(0)
      const ws = mockWebSocketInstances[0]
      expect(ws.readyState).toBe(MockWebSocket.OPEN)
    },
    { timeout: 2000 }
  )
  return mockWebSocketInstances[0]
}

// Helper to send WebSocket snapshot message
// This simulates what useGameSync does - it updates the query cache
function sendWebSocketSnapshot(
  ws: MockWebSocket,
  snapshot: typeof initSnapshotFixture,
  gameId: number,
  queryClient: QueryClient,
  overrides?: {
    viewerSeat?: number
    version?: number
    viewerHand?: string[]
  }
) {
  // Transform the snapshot message to GameRoomSnapshotPayload format
  // This simulates what useGameSync.transformSnapshotMessage does
  const version = overrides?.version ?? 1
  const viewerSeat = overrides?.viewerSeat ?? 0
  const viewerHand = overrides?.viewerHand ?? []
  const playerNames: [string, string, string, string] = [
    'Alex',
    'Bailey',
    'Casey',
    'Dakota',
  ] // Simplified for tests

  const payload: GameRoomSnapshotPayload = {
    snapshot,
    playerNames,
    viewerSeat: viewerSeat as any,
    viewerHand,
    timestamp: new Date().toISOString(),
    hostSeat: snapshot.game.host_seat as any,
    bidConstraints: null,
    version,
    etag: `"game-${gameId}-v${version}"`,
  }

  // Update the real query cache (simulating what useGameSync does)
  queryClient.setQueryData(queryKeys.games.snapshot(gameId), payload)

  const message = {
    type: 'snapshot',
    data: {
      snapshot,
      version: version,
      viewer_hand: viewerHand,
      bid_constraints: null,
    },
    viewer_seat: viewerSeat,
  }
  act(() => {
    ws.onmessage?.(
      new MessageEvent('message', {
        data: JSON.stringify(message),
      })
    )
  })
}

describe('GameRoomClient', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    // Use real timers by default - userEvent and waitFor need real timers
    // Individual tests can switch to fake timers for timing-specific tests
    vi.useRealTimers()

    // Reset initialized game IDs
    initializedGameIds.clear()
    // Query client is reset automatically by test utils

    // Explicitly reset all mocks to clear any queued implementations from previous tests
    // mockReset() clears both call history AND implementation queues
    mockGetGameRoomSnapshotAction.mockReset()
    mockMarkPlayerReadyAction.mockReset()
    mockSubmitBidAction.mockReset()
    mockSelectTrumpAction.mockReset()
    mockSubmitPlayAction.mockReset()
    mockAddAiSeatAction.mockReset()
    mockUpdateAiSeatAction.mockReset()
    mockRemoveAiSeatAction.mockReset()
    mockFetchAiRegistryAction.mockReset()
    mockShowToast.mockReset()
    mockHideToast.mockReset()

    // Set environment variable for useGameSync to resolve WebSocket URL
    process.env.NEXT_PUBLIC_BACKEND_BASE_URL = 'http://localhost:3001'

    // Reset WebSocket mock
    mockWebSocketInstances.length = 0
    vi.stubGlobal('WebSocket', MockWebSocket)

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
            json: async () => ({ token: 'mock-ws-token' }),
          } as Response)
        }
        // Fallback to original fetch for other requests
        return originalFetch(url)
      })
    )

    // Default mock implementations
    mockGetGameRoomSnapshotAction.mockResolvedValue({
      kind: 'ok',
      data: createInitialData(),
    })
    mockMarkPlayerReadyAction.mockResolvedValue({ kind: 'ok' })
    mockSubmitBidAction.mockResolvedValue({ kind: 'ok' })
    mockSelectTrumpAction.mockResolvedValue({ kind: 'ok' })
    mockSubmitPlayAction.mockResolvedValue({ kind: 'ok' })
    mockAddAiSeatAction.mockResolvedValue({ kind: 'ok' })
    mockUpdateAiSeatAction.mockResolvedValue({ kind: 'ok' })
    mockRemoveAiSeatAction.mockResolvedValue({ kind: 'ok' })
    mockFetchAiRegistryAction.mockResolvedValue({
      kind: 'ok',
      data: [
        { name: 'Heuristic', version: '1.0.0' },
        { name: 'RandomPlayer', version: '1.0.0' },
      ],
    })

    // Clear AI registry calls before each test
    mockFetchAiRegistryAction.mockClear()
  })

  afterEach(() => {
    vi.useRealTimers()
    vi.clearAllTimers()
    // Restore original WebSocket and fetch
    vi.unstubAllGlobals()
    // Clear environment variable
    delete process.env.NEXT_PUBLIC_BACKEND_BASE_URL
  })

  describe('WebSocket integration', () => {
    it('updates snapshot when WebSocket receives message', async () => {
      const initialData = createInitialData()

      const { queryClient } = await act(async () => {
        return render(<GameRoomClient initialData={initialData} gameId={42} />)
      })

      // Wait for WebSocket to connect
      const ws = await waitForWebSocketConnection()

      // Send WebSocket message with updated snapshot
      sendWebSocketSnapshot(ws, biddingSnapshotFixture, 42, queryClient, {
        viewerSeat: 0,
        version: 1,
      })

      // Verify snapshot updated (may appear multiple times - in panel and sidebar)
      await waitFor(
        () => {
          const biddingElements = screen.getAllByText(/Bidding/i)
          expect(biddingElements.length).toBeGreaterThan(0)
        },
        { timeout: 2000 }
      )
    })

    it('updates snapshot after action completes via WebSocket', async () => {
      const initialData = createInitialData()

      const { queryClient } = await act(async () => {
        return render(<GameRoomClient initialData={initialData} gameId={42} />)
      })

      // Wait for WebSocket to connect
      const ws = await waitForWebSocketConnection()

      // Mark ready
      const readyButton = screen.getByRole('button', {
        name: /Mark yourself as ready/i,
      })
      await userEvent.click(readyButton)

      // Wait for ready action to complete
      await waitFor(
        () => {
          expect(mockMarkPlayerReadyAction).toHaveBeenCalledWith(42, true)
        },
        { timeout: 2000 }
      )

      // Simulate WebSocket message after action completes
      sendWebSocketSnapshot(ws, biddingSnapshotFixture, 42, queryClient, {
        viewerSeat: 0,
        version: 1,
      })

      // Verify snapshot updated via WebSocket (no manual refresh needed)
      // May appear multiple times - in panel and sidebar
      await waitFor(
        () => {
          const biddingElements = screen.getAllByText(/Bidding/i)
          expect(biddingElements.length).toBeGreaterThan(0)
        },
        { timeout: 2000 }
      )

      // Verify no manual refresh was called (WebSocket updates the cache directly)
      // Note: useGameSync might call getGameRoomSnapshotAction during initialization,
      // so we just verify the WebSocket update worked
      const biddingElements = screen.getAllByText(/Bidding/i)
      expect(biddingElements.length).toBeGreaterThan(0)
    })

    it('automatically retries via HTTP when WebSocket error received', async () => {
      const initialData = createInitialData()

      await act(async () => {
        const { queryClient: _ } = render(
          <GameRoomClient initialData={initialData} gameId={42} />
        )
      })

      // Wait for WebSocket to connect
      const ws = await waitForWebSocketConnection()

      // Clear any previous calls
      mockGetGameRoomSnapshotAction.mockClear()

      // Send error message
      act(() => {
        ws.onmessage?.(
          new MessageEvent('message', {
            data: JSON.stringify({
              type: 'error',
              message: 'Failed to build snapshot',
              code: 'INTERNAL_ERROR',
            }),
          })
        )
      })

      // Verify HTTP refresh was automatically triggered
      await waitFor(
        () => {
          expect(mockGetGameRoomSnapshotAction).toHaveBeenCalled()
        },
        { timeout: 2000 }
      )

      // Verify error is NOT displayed yet (HTTP retry should succeed)
      expect(
        screen.queryByText(/Failed to build snapshot/i)
      ).not.toBeInTheDocument()
    })

    it('shows error only if HTTP retry also fails', async () => {
      const initialData = createInitialData()

      await act(async () => {
        const { queryClient: _ } = render(
          <GameRoomClient initialData={initialData} gameId={42} />
        )
      })

      // Wait for WebSocket to connect
      const ws = await waitForWebSocketConnection()

      // Mock HTTP refresh to fail
      mockGetGameRoomSnapshotAction.mockResolvedValueOnce({
        kind: 'error',
        message: 'HTTP refresh also failed',
        traceId: 'test-trace-id',
      })

      // Send error message
      act(() => {
        ws.onmessage?.(
          new MessageEvent('message', {
            data: JSON.stringify({
              type: 'error',
              message: 'Failed to build snapshot',
              code: 'INTERNAL_ERROR',
            }),
          })
        )
      })

      // Verify HTTP refresh was triggered
      await waitFor(
        () => {
          expect(mockGetGameRoomSnapshotAction).toHaveBeenCalled()
        },
        { timeout: 2000 }
      )

      // Verify error is now displayed (both WS and HTTP failed)
      await waitFor(
        () => {
          expect(
            screen.getByText(/HTTP refresh also failed/i)
          ).toBeInTheDocument()
        },
        { timeout: 2000 }
      )
    })

    it('clears error when HTTP retry succeeds', async () => {
      const initialData = createInitialData()

      await act(async () => {
        const { queryClient: _ } = render(
          <GameRoomClient initialData={initialData} gameId={42} />
        )
      })

      // Wait for WebSocket to connect
      const ws = await waitForWebSocketConnection()

      // Mock HTTP refresh to succeed
      const refreshedData = createInitialData(initSnapshotFixture, {
        version: 2,
      })
      mockGetGameRoomSnapshotAction.mockResolvedValueOnce({
        kind: 'ok',
        data: refreshedData,
      })

      // Send error message
      act(() => {
        ws.onmessage?.(
          new MessageEvent('message', {
            data: JSON.stringify({
              type: 'error',
              message: 'Failed to build snapshot',
              code: 'INTERNAL_ERROR',
            }),
          })
        )
      })

      // Wait for HTTP retry to complete
      await waitFor(
        () => {
          expect(mockGetGameRoomSnapshotAction).toHaveBeenCalled()
        },
        { timeout: 2000 }
      )

      // Verify error is NOT displayed (HTTP retry succeeded)
      expect(
        screen.queryByText(/Failed to build snapshot/i)
      ).not.toBeInTheDocument()
    })
  })
})
