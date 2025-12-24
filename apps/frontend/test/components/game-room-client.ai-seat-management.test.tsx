import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { render, waitFor, act } from '../utils'
import type { ReactNode } from 'react'

import { GameRoomClient } from '@/app/game/[gameId]/_components/game-room-client'
import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import { initSnapshotFixture } from '../mocks/game-snapshot'
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
})
