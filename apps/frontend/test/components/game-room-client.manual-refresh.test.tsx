import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import {
  render,
  screen,
  waitFor,
  act,
  fireEvent,
  createTestQueryClient,
} from '../utils'
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

  describe('Manual refresh', () => {
    it('updates snapshot on successful refresh', async () => {
      const initialData = createInitialData()
      const newSnapshot = { ...biddingSnapshotFixture }
      const newData = createInitialData(newSnapshot, { etag: 'new-etag' })

      mockGetGameRoomSnapshotAction.mockResolvedValue({
        kind: 'ok',
        data: newData,
      })

      await act(async () => {
        render(<GameRoomClient initialData={initialData} gameId={42} />)
      })

      // There might be multiple refresh buttons, get the first one
      const refreshButtons = screen.getAllByRole('button', {
        name: /Refresh game state/i,
      })
      await userEvent.click(refreshButtons[0])

      // Wait for async operations to complete
      await waitFor(
        () => {
          expect(mockGetGameRoomSnapshotAction).toHaveBeenCalled()
        },
        { timeout: 2000 }
      )

      expect(mockGetGameRoomSnapshotAction).toHaveBeenCalledWith({
        gameId: 42,
        etag: 'initial-etag',
      })

      // Should update to bidding phase
      expect(screen.getByText(/Bidding/i)).toBeInTheDocument()
    })

    it('handles ETag not modified response', async () => {
      const initialData = createInitialData()

      mockGetGameRoomSnapshotAction.mockResolvedValue({
        kind: 'not_modified',
      })

      await act(async () => {
        render(<GameRoomClient initialData={initialData} gameId={42} />)
      })

      // There might be multiple refresh buttons, get the first one
      const refreshButtons = screen.getAllByRole('button', {
        name: /Refresh game state/i,
      })
      await userEvent.click(refreshButtons[0])

      // Wait for async operations to complete
      await waitFor(
        () => {
          expect(mockGetGameRoomSnapshotAction).toHaveBeenCalled()
        },
        { timeout: 2000 }
      )

      expect(mockGetGameRoomSnapshotAction).toHaveBeenCalled()

      // Should still render (not crash)
      expect(screen.getByText(/Init/i)).toBeInTheDocument()
    })

    it('handles refresh errors', async () => {
      const initialData = createInitialData()

      mockGetGameRoomSnapshotAction.mockResolvedValue({
        kind: 'error',
        message: 'Network error',
        status: 500,
        traceId: 'trace-123',
      })

      await act(async () => {
        render(<GameRoomClient initialData={initialData} gameId={42} />)
      })

      // There might be multiple refresh buttons, get the first one
      const refreshButtons = screen.getAllByRole('button', {
        name: /Refresh game state/i,
      })
      await userEvent.click(refreshButtons[0])

      await waitFor(
        () => {
          expect(screen.getByText(/Network error/i)).toBeInTheDocument()
        },
        { timeout: 2000 }
      )
    })

    it('prevents concurrent refresh calls', async () => {
      const initialData = createInitialData()

      // Initialize query cache before rendering to ensure data is available immediately
      const queryClient = createTestQueryClient()
      queryClient.setQueryData(queryKeys.games.snapshot(42), initialData)

      await act(async () => {
        render(<GameRoomClient initialData={initialData} gameId={42} />, {
          queryClient,
        })
      })

      // Wait for WebSocket to connect
      await waitForWebSocketConnection()

      // Wait for component to render (check for Init phase text)
      await waitFor(
        () => {
          expect(screen.getByText(/Init/i)).toBeInTheDocument()
        },
        { timeout: 3000 }
      )

      // Wait for refresh button to appear - try multiple ways to find it
      const refreshButton = await waitFor(
        () => {
          // Try by aria-label first
          const buttons = screen.queryAllByRole('button', {
            name: /Refresh game state/i,
          })
          if (buttons.length > 0) {
            return buttons[0]
          }
          // Fallback: try by text content
          const buttonsByText = screen
            .getAllByRole('button')
            .filter(
              (btn) =>
                btn.textContent?.includes('Refresh game state') ||
                btn.textContent?.includes('Manual sync')
            )
          if (buttonsByText.length > 0) {
            return buttonsByText[0]
          }
          throw new Error('Refresh button not found')
        },
        { timeout: 3000 }
      )

      // Clear any calls from initialization
      mockGetGameRoomSnapshotAction.mockClear()

      // Make the first call hang - set up the mock to return a hanging promise
      let resolveFirst: () => void
      const firstPromise = new Promise<{
        kind: 'ok'
        data: GameRoomSnapshotPayload
      }>((resolve) => {
        resolveFirst = () => resolve({ kind: 'ok', data: createInitialData() })
      })
      // Set up the mock to return the hanging promise for the refresh call
      mockGetGameRoomSnapshotAction.mockImplementationOnce(() => firstPromise)

      // Trigger first refresh
      await userEvent.click(refreshButton)

      // Wait a bit to ensure the first refresh has started
      await waitFor(
        () => {
          expect(mockGetGameRoomSnapshotAction).toHaveBeenCalledTimes(1)
        },
        { timeout: 2000 }
      )

      // Trigger second refresh while first is in progress
      await userEvent.click(refreshButton)

      // Second click should not trigger another call (concurrent prevention works)
      expect(mockGetGameRoomSnapshotAction).toHaveBeenCalledTimes(1)

      // Resolve the first call
      await act(async () => {
        resolveFirst!()
        await firstPromise
      })

      // After first completes, only one call should have been made
      expect(mockGetGameRoomSnapshotAction).toHaveBeenCalledTimes(1)
    })

    it('shows slow sync indicator when manual refresh takes longer than 1 second', async () => {
      const initialData = createInitialData()

      // Use real timers for WebSocket connection
      vi.useRealTimers()
      mockShowToast.mockReturnValueOnce('slow-sync-id')

      // Initialize query cache BEFORE rendering to ensure data is available immediately
      const queryClient = createTestQueryClient()
      queryClient.setQueryData(queryKeys.games.snapshot(42), initialData)

      await act(async () => {
        render(<GameRoomClient initialData={initialData} gameId={42} />, {
          queryClient,
        })
      })

      // Wait for WebSocket to connect
      await waitForWebSocketConnection()

      // Wait for component to render (check for Init phase text)
      await waitFor(
        () => {
          expect(screen.getByText(/Init/i)).toBeInTheDocument()
        },
        { timeout: 3000 }
      )

      // Wait for refresh button to appear - try multiple ways to find it
      const refreshButton = await waitFor(
        () => {
          // Try by aria-label first
          const buttons = screen.queryAllByRole('button', {
            name: /Refresh game state/i,
          })
          if (buttons.length > 0) {
            return buttons[0]
          }
          // Fallback: try by text content
          const buttonsByText = screen
            .getAllByRole('button')
            .filter(
              (btn) =>
                btn.textContent?.includes('Refresh game state') ||
                btn.textContent?.includes('Manual sync')
            )
          if (buttonsByText.length > 0) {
            return buttonsByText[0]
          }
          throw new Error('Refresh button not found')
        },
        { timeout: 3000 }
      )

      // Clear any calls from initialization
      mockGetGameRoomSnapshotAction.mockClear()

      // Make refresh slow (longer than 1 second) - set up the mock to return a hanging promise
      let resolveRefresh: () => void
      const refreshPromise = new Promise<{
        kind: 'ok'
        data: GameRoomSnapshotPayload
      }>((resolve) => {
        resolveRefresh = () =>
          resolve({
            kind: 'ok',
            data: createInitialData(),
          })
      })
      mockGetGameRoomSnapshotAction.mockReturnValueOnce(refreshPromise)

      // Now switch to fake timers for the rest of the test
      vi.useFakeTimers()

      await act(async () => {
        fireEvent.click(refreshButton)
        // Advance time by 1 second - slow sync indicator should appear
        await vi.advanceTimersByTimeAsync(1000)
      })

      // Check for slow sync indicator (toast)
      expect(mockShowToast).toHaveBeenCalledWith(
        'Updating game state…',
        'warning'
      )

      // Resolve the refresh
      await act(async () => {
        resolveRefresh!()
        await refreshPromise
        await vi.advanceTimersByTimeAsync(0)
      })

      // Slow sync indicator should disappear
      await act(async () => {
        await vi.advanceTimersByTimeAsync(0)
      })
      expect(mockHideToast).toHaveBeenCalledWith('slow-sync-id')

      vi.useRealTimers()
    })
  })
})
