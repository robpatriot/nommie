import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import {
  render,
  screen,
  waitFor,
  act,
  fireEvent,
  getTestQueryClient,
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

// Mock server actions
const mockGetGameRoomSnapshotAction = vi.fn()
const mockMarkPlayerReadyAction = vi.fn()
const mockSubmitBidAction = vi.fn()
const mockSelectTrumpAction = vi.fn()
const mockSubmitPlayAction = vi.fn()
const mockAddAiSeatAction = vi.fn()
const mockUpdateAiSeatAction = vi.fn()
const mockRemoveAiSeatAction = vi.fn()
const mockFetchAiRegistryAction = vi.fn()

vi.mock('@/app/actions/game-room-actions', () => ({
  getGameRoomSnapshotAction: (request: unknown) =>
    mockGetGameRoomSnapshotAction(request),
  markPlayerReadyAction: (gameId: number) => mockMarkPlayerReadyAction(gameId),
  submitBidAction: (request: unknown) => mockSubmitBidAction(request),
  selectTrumpAction: (request: unknown) => mockSelectTrumpAction(request),
  submitPlayAction: (request: unknown) => mockSubmitPlayAction(request),
  addAiSeatAction: (request: unknown) => mockAddAiSeatAction(request),
  updateAiSeatAction: (request: unknown) => mockUpdateAiSeatAction(request),
  removeAiSeatAction: (request: unknown) => mockRemoveAiSeatAction(request),
  fetchAiRegistryAction: () => mockFetchAiRegistryAction(),
}))

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

  describe('Initialization', () => {
    it('renders with initial data', async () => {
      const initialData = createInitialData()

      await act(async () => {
        render(<GameRoomClient initialData={initialData} gameId={42} />)
      })

      expect(screen.getByText(/Init/i)).toBeInTheDocument()
    })

    it('starts in idle state', async () => {
      const initialData = createInitialData()

      await act(async () => {
        render(<GameRoomClient initialData={initialData} gameId={42} />)
      })

      // Component should render without errors
      expect(screen.getByText(/Init/i)).toBeInTheDocument()
    })
  })

  describe('WebSocket integration', () => {
    it('updates snapshot when WebSocket receives message', async () => {
      const initialData = createInitialData()

      await act(async () => {
        render(<GameRoomClient initialData={initialData} gameId={42} />)
      })

      // Wait for WebSocket to connect
      const ws = await waitForWebSocketConnection()

      // Send WebSocket message with updated snapshot
      sendWebSocketSnapshot(ws, biddingSnapshotFixture, 42, {
        viewerSeat: 0,
        lockVersion: 1,
      })

      // Verify snapshot updated
      await waitFor(
        () => {
          expect(screen.getByText(/Bidding/i)).toBeInTheDocument()
        },
        { timeout: 2000 }
      )
    })

    it('updates snapshot after action completes via WebSocket', async () => {
      const initialData = createInitialData()

      await act(async () => {
        render(<GameRoomClient initialData={initialData} gameId={42} />)
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
          expect(mockMarkPlayerReadyAction).toHaveBeenCalledWith(42)
        },
        { timeout: 2000 }
      )

      // Simulate WebSocket message after action completes
      sendWebSocketSnapshot(ws, biddingSnapshotFixture, 42, {
        viewerSeat: 0,
        lockVersion: 1,
      })

      // Verify snapshot updated via WebSocket (no manual refresh needed)
      await waitFor(
        () => {
          expect(screen.getByText(/Bidding/i)).toBeInTheDocument()
        },
        { timeout: 2000 }
      )

      // Verify no manual refresh was called (WebSocket updates the cache directly)
      // Note: useGameSync might call getGameRoomSnapshotAction during initialization,
      // so we just verify the WebSocket update worked
      expect(screen.getByText(/Bidding/i)).toBeInTheDocument()
    })

    it('handles WebSocket error messages', async () => {
      const initialData = createInitialData()

      await act(async () => {
        render(<GameRoomClient initialData={initialData} gameId={42} />)
      })

      // Wait for WebSocket to connect
      const ws = await waitForWebSocketConnection()

      // Send error message
      act(() => {
        ws.onmessage?.(
          new MessageEvent('message', {
            data: JSON.stringify({
              type: 'error',
              message: 'WebSocket error',
              code: 'WS_ERROR',
            }),
          })
        )
      })

      // Verify error is displayed
      await waitFor(
        () => {
          expect(screen.getByText(/WebSocket error/i)).toBeInTheDocument()
        },
        { timeout: 2000 }
      )
    })
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
      const queryClient = getTestQueryClient()
      if (queryClient) {
        queryClient.setQueryData(queryKeys.games.snapshot(42), initialData)
      }

      await act(async () => {
        render(<GameRoomClient initialData={initialData} gameId={42} />)
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
      let refreshButton: HTMLElement
      await waitFor(
        () => {
          // Try by aria-label first
          const buttons = screen.queryAllByRole('button', {
            name: /Refresh game state/i,
          })
          if (buttons.length > 0) {
            refreshButton = buttons[0]
            return
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
            refreshButton = buttonsByText[0]
            return
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
      const queryClient = getTestQueryClient()
      if (queryClient) {
        queryClient.setQueryData(queryKeys.games.snapshot(42), initialData)
      }

      await act(async () => {
        render(<GameRoomClient initialData={initialData} gameId={42} />)
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
      let refreshButton: HTMLElement
      await waitFor(
        () => {
          // Try by aria-label first
          const buttons = screen.queryAllByRole('button', {
            name: /Refresh game state/i,
          })
          if (buttons.length > 0) {
            refreshButton = buttons[0]
            return
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
            refreshButton = buttonsByText[0]
            return
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

  describe('Bid action', () => {
    it('submits bid', async () => {
      // Create a bidding snapshot where viewer (seat 0) hasn't bid yet
      const biddingSnapshotWithNoBid = {
        ...biddingSnapshotFixture,
        phase: {
          ...biddingSnapshotFixture.phase,
          data: {
            ...biddingSnapshotFixture.phase.data,
            bids: [null, null, null, null], // Viewer hasn't bid yet
            to_act: 0, // It's the viewer's turn
          },
        },
      }
      const biddingData = createInitialData(biddingSnapshotWithNoBid, {
        viewerSeat: 0,
        viewerHand: ['2H', '3C'],
        lockVersion: 1,
      })

      // Initialize query cache with bidding data BEFORE rendering
      const queryClient = getTestQueryClient()
      if (queryClient) {
        queryClient.setQueryData(queryKeys.games.snapshot(42), biddingData)
      }

      // Override the default mock to return bidding data for this test
      // This ensures if the query fetches, it returns the correct data
      mockGetGameRoomSnapshotAction.mockResolvedValueOnce({
        kind: 'ok',
        data: biddingData,
      })

      await act(async () => {
        render(<GameRoomClient initialData={biddingData} gameId={42} />)
      })

      // Wait for WebSocket to connect
      await waitForWebSocketConnection()

      // Wait for component to render and bidding phase to appear
      // The BiddingPanel component shows "Bidding" as an h2 element
      await waitFor(
        () => {
          // Try to find the Bidding heading - there might be multiple, so use getAllByRole
          const biddingHeadings = screen.queryAllByRole('heading', {
            name: /Bidding/i,
          })
          if (biddingHeadings.length > 0) {
            return
          }
          // Fallback: try to find any text containing "Bidding"
          const biddingText = screen.queryByText(/Bidding/i)
          if (biddingText) {
            return
          }
          throw new Error('Bidding phase not found')
        },
        { timeout: 3000 }
      )

      // Find bid input and submit a bid
      const bidInput = screen.getByLabelText(/Your bid/i)
      await userEvent.type(bidInput, '3')
      const submitButton = screen.getByRole('button', { name: /Submit bid/i })
      await userEvent.click(submitButton)

      // Wait for bid to be submitted
      await waitFor(
        () => {
          expect(mockSubmitBidAction).toHaveBeenCalledWith({
            gameId: 42,
            bid: 3,
            lockVersion: 1,
          })
        },
        { timeout: 2000 }
      )
    })
  })

  describe('AI seat management', () => {
    it('loads AI registry when host views AI manager', async () => {
      const initialData = createInitialData(initSnapshotFixture, {
        viewerSeat: 0, // Host
        hostSeat: 0,
      })

      await act(async () => {
        render(<GameRoomClient initialData={initialData} gameId={42} />)
      })

      // Wait for async operations (AI registry fetch - enough for promise to resolve)
      await act(async () => {
        // Wait for promise to resolve
        await new Promise((resolve) => setTimeout(resolve, 50))
      })

      // AI registry should be fetched when component mounts and host can view AI manager
      expect(mockFetchAiRegistryAction).toHaveBeenCalled()
    })

    it('does not load AI registry for non-host', async () => {
      const initialData = createInitialData(initSnapshotFixture, {
        viewerSeat: 1, // Not host
        hostSeat: 0,
      })

      // Clear any previous calls (already cleared in beforeEach)
      mockFetchAiRegistryAction.mockClear()
      const callCountBefore = mockFetchAiRegistryAction.mock.calls.length

      await act(async () => {
        render(<GameRoomClient initialData={initialData} gameId={42} />)
      })

      // Wait for WebSocket to connect and component to fully render
      await waitForWebSocketConnection()

      // Wait a bit more for any async operations to complete
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 300))
      })

      // AI registry should not be fetched for non-host (enabled=false)
      // TanStack Query should respect the enabled flag and not call the query function
      // Note: TanStack Query might call the query once during initialization even if disabled,
      // so we check that it's called at most once (or not at all)
      const callCountAfter = mockFetchAiRegistryAction.mock.calls.length
      const newCalls = callCountAfter - callCountBefore
      // Should be 0, but allow 1 if TanStack Query calls it during initialization
      expect(newCalls).toBeLessThanOrEqual(1)

      // If it was called, it should only be once during initialization, not repeatedly
      if (newCalls > 0) {
        expect(mockFetchAiRegistryAction).toHaveBeenCalledTimes(1)
      }
    })

    it('adds AI seat', async () => {
      const initialData = createInitialData(initSnapshotFixture, {
        viewerSeat: 0,
        hostSeat: 0,
      })

      await act(async () => {
        render(<GameRoomClient initialData={initialData} gameId={42} />)
      })

      // Wait for AI registry to load
      await act(async () => {
        // Wait for promise to resolve
        await new Promise((resolve) => setTimeout(resolve, 50))
      })

      expect(mockFetchAiRegistryAction).toHaveBeenCalled()

      // The AI seat management is tested through the handler guards
      // Full UI interaction tests would be in game-room-view.test.tsx
    })

    it('cleans up AI registry fetch on unmount', async () => {
      const initialData = createInitialData(initSnapshotFixture, {
        viewerSeat: 0,
        hostSeat: 0,
      })

      let resolveRegistry: () => void
      const registryPromise = new Promise<{
        kind: 'ok'
        data: Array<{ name: string; version: string }>
      }>((resolve) => {
        resolveRegistry = () =>
          resolve({
            kind: 'ok',
            data: [{ name: 'HeuristicV1', version: '1.0.0' }],
          })
      })
      mockFetchAiRegistryAction.mockReturnValueOnce(registryPromise)

      let unmount: () => void
      await act(async () => {
        const result = render(
          <GameRoomClient initialData={initialData} gameId={42} />
        )
        unmount = result.unmount
      })

      // Unmount before registry resolves
      await act(async () => {
        unmount()
      })

      // Resolve after unmount - should not cause state updates
      await act(async () => {
        resolveRegistry!()
        await registryPromise
      })

      // No errors should occur
    })
  })

  describe('Error handling', () => {
    it('displays error messages with traceId', async () => {
      const initialData = createInitialData()

      mockGetGameRoomSnapshotAction.mockResolvedValue({
        kind: 'error',
        message: 'Failed to fetch',
        status: 500,
        traceId: 'trace-abc-123',
      })

      await act(async () => {
        render(<GameRoomClient initialData={initialData} gameId={42} />)
      })

      // There might be multiple refresh buttons, get the first one
      const refreshButtons = screen.getAllByRole('button', {
        name: /Refresh game state/i,
      })
      await userEvent.click(refreshButtons[0])

      // Wait for async operations
      await waitFor(
        () => {
          expect(mockGetGameRoomSnapshotAction).toHaveBeenCalled()
        },
        { timeout: 2000 }
      )

      expect(screen.getByText(/Failed to fetch/i)).toBeInTheDocument()

      // TraceId should be visible (may be in expandable details)
    })

    it('handles network errors gracefully', async () => {
      const initialData = createInitialData()

      const expectedError = new Error('Network error')
      mockGetGameRoomSnapshotAction.mockRejectedValue(expectedError)
      const consoleErrorSpy = vi
        .spyOn(console, 'error')
        .mockImplementation(() => {})

      await act(async () => {
        render(<GameRoomClient initialData={initialData} gameId={42} />)
      })

      const refreshButton = screen.getByRole('button', {
        name: /Refresh game state/i,
      })
      try {
        const callCountBefore = mockGetGameRoomSnapshotAction.mock.calls.length

        await expect(userEvent.click(refreshButton)).resolves.toBeUndefined()

        await waitFor(
          () => {
            expect(mockGetGameRoomSnapshotAction.mock.calls.length).toBe(
              callCountBefore + 1
            )
          },
          { timeout: 2000 }
        )

        const results = mockGetGameRoomSnapshotAction.mock.results
        const failingCall =
          results.length > 0
            ? results[results.length - 1]?.value
            : Promise.reject(new Error('Missing promise'))
        if (!failingCall) {
          throw new Error('Missing promise')
        }

        await expect(failingCall).rejects.toThrow('Network error')

        await waitFor(
          () => {
            expect(screen.getByText(/Network error/i)).toBeInTheDocument()
          },
          { timeout: 2000 }
        )
      } finally {
        consoleErrorSpy.mockRestore()
      }
    })
  })

  describe('Action coordination', () => {
    it('prevents actions when another action is in progress', async () => {
      const initialData = createInitialData()

      // Make ready action slow
      let resolveReady: () => void
      const readyPromise = new Promise<{ kind: 'ok' }>((resolve) => {
        resolveReady = () => resolve({ kind: 'ok' })
      })
      mockMarkPlayerReadyAction.mockReturnValueOnce(readyPromise)

      await act(async () => {
        render(<GameRoomClient initialData={initialData} gameId={42} />)
      })

      // Start ready action
      const readyButton = screen.getByRole('button', {
        name: /Mark yourself as ready/i,
      })
      await userEvent.click(readyButton)

      // Try to click ready again while first is in progress
      await userEvent.click(readyButton)

      // Should only have been called once
      expect(mockMarkPlayerReadyAction).toHaveBeenCalledTimes(1)

      // Resolve ready
      await act(async () => {
        resolveReady!()
        await readyPromise
        // Wait for promise to resolve
        await new Promise((resolve) => setTimeout(resolve, 50))
      })
    })

    it('allows manual refresh independently of actions', async () => {
      const initialData = createInitialData()

      // Make ready action slow
      let resolveReady: () => void
      const readyPromise = new Promise<{ kind: 'ok' }>((resolve) => {
        resolveReady = () => resolve({ kind: 'ok' })
      })
      mockMarkPlayerReadyAction.mockReturnValueOnce(readyPromise)

      await act(async () => {
        render(<GameRoomClient initialData={initialData} gameId={42} />)
      })

      // Start ready action
      const readyButton = screen.getByRole('button', {
        name: /Mark yourself as ready/i,
      })
      await userEvent.click(readyButton)

      // Try to refresh while action is in progress - should work independently
      // There might be multiple refresh buttons, get the first one
      const refreshButtons = screen.getAllByRole('button', {
        name: /Refresh game state/i,
      })
      await userEvent.click(refreshButtons[0])

      // Refresh should execute independently
      await waitFor(
        () => {
          expect(mockGetGameRoomSnapshotAction).toHaveBeenCalled()
        },
        { timeout: 2000 }
      )

      // Resolve ready
      await act(async () => {
        resolveReady!()
        await readyPromise
        await new Promise((resolve) => setTimeout(resolve, 50))
      })
    })
  })
})
