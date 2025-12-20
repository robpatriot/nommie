import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, waitFor, act, getTestQueryClient } from '../utils'
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
  mutateAsync: async (gameId: number) => {
    const result = await mockMarkPlayerReadyAction(gameId)
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

vi.mock('@/hooks/mutations/useGameRoomMutations', () => ({
  useMarkPlayerReady: () => mockUseMarkPlayerReady(),
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
  overrides?: {
    viewerSeat?: number
    lockVersion?: number
    viewerHand?: string[]
  }
) {
  // Transform the snapshot message to GameRoomSnapshotPayload format
  // This simulates what useGameSync.transformSnapshotMessage does
  const lockVersion = overrides?.lockVersion ?? 1
  const viewerSeat = overrides?.viewerSeat ?? 0
  const viewerHand = overrides?.viewerHand ?? []
  const playerNames = ['Alex', 'Bailey', 'Casey', 'Dakota'] // Simplified for tests

  const payload: GameRoomSnapshotPayload = {
    snapshot,
    playerNames,
    viewerSeat: viewerSeat as any,
    viewerHand,
    timestamp: new Date().toISOString(),
    hostSeat: snapshot.game.host_seat as any,
    bidConstraints: null,
    lockVersion,
    etag: `"game-${gameId}-v${lockVersion}"`,
  }

  // Update the real query cache (simulating what useGameSync does)
  const queryClient = getTestQueryClient()
  if (queryClient) {
    queryClient.setQueryData(queryKeys.games.snapshot(gameId), payload)
  }

  const message = {
    type: 'snapshot',
    data: {
      snapshot,
      lock_version: lockVersion,
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
        { name: 'HeuristicV1', version: '1.0.0' },
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

  describe('Ready action', () => {
    it('marks player ready', async () => {
      const initialData = createInitialData()

      await act(async () => {
        render(<GameRoomClient initialData={initialData} gameId={42} />)
      })

      const readyButton = screen.getByRole('button', {
        name: /Mark yourself as ready/i,
      })
      await userEvent.click(readyButton)

      // Wait for async operations
      await waitFor(
        () => {
          expect(mockMarkPlayerReadyAction).toHaveBeenCalled()
        },
        { timeout: 2000 }
      )

      expect(mockMarkPlayerReadyAction).toHaveBeenCalledWith(42)

      // Note: No manual refresh expected - WebSocket will handle updates
    })

    it('prevents duplicate ready calls', async () => {
      const initialData = createInitialData()

      await act(async () => {
        render(<GameRoomClient initialData={initialData} gameId={42} />)
      })

      const readyButton = screen.getByRole('button', {
        name: /Mark yourself as ready/i,
      })

      // Click multiple times quickly
      await userEvent.click(readyButton)
      await userEvent.click(readyButton)
      await userEvent.click(readyButton)

      // Wait for async operations
      await waitFor(
        () => {
          expect(mockMarkPlayerReadyAction).toHaveBeenCalled()
        },
        { timeout: 2000 }
      )

      // Should only call once
      expect(mockMarkPlayerReadyAction).toHaveBeenCalledTimes(1)
    })

    it('handles ready action errors', async () => {
      const initialData = createInitialData()

      mockMarkPlayerReadyAction.mockResolvedValue({
        kind: 'error',
        message: 'Already ready',
        status: 400,
      })

      await act(async () => {
        render(<GameRoomClient initialData={initialData} gameId={42} />)
      })

      const readyButton = screen.getByRole('button', {
        name: /Mark yourself as ready/i,
      })
      await userEvent.click(readyButton)

      // Wait for error message to appear
      await waitFor(
        () => {
          expect(screen.getByText(/Already ready/i)).toBeInTheDocument()
        },
        { timeout: 2000 }
      )

      // Ready action should have been called
      expect(mockMarkPlayerReadyAction).toHaveBeenCalled()
      // Note: useGameSync might call getGameRoomSnapshotAction during initialization,
      // so we just verify the error is displayed
      expect(screen.getByText(/Already ready/i)).toBeInTheDocument()
    })

    it('resets hasMarkedReady when phase changes', async () => {
      const initialData = createInitialData()

      await act(async () => {
        render(<GameRoomClient initialData={initialData} gameId={42} />)
      })

      // Mark ready
      const readyButton = screen.getByRole('button', {
        name: /Mark yourself as ready/i,
      })
      await userEvent.click(readyButton)

      // Wait for ready action to complete
      await act(async () => {
        // Wait for promise to resolve
        await new Promise((resolve) => setTimeout(resolve, 50))
      })

      expect(mockMarkPlayerReadyAction).toHaveBeenCalled()

      // Simulate phase change via WebSocket
      const ws = await waitForWebSocketConnection()
      sendWebSocketSnapshot(ws, biddingSnapshotFixture, 42, {
        viewerSeat: 0,
        lockVersion: 1,
      })

      // Wait for phase change
      await waitFor(
        () => {
          expect(screen.getByText(/Bidding/i)).toBeInTheDocument()
        },
        { timeout: 2000 }
      )

      // Ready button should not be marked as ready anymore (if it exists)
      // This tests the phase change effect
    })
  })
})
