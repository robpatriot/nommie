import { vi } from 'vitest'
import {
  mockGetGameRoomStateAction,
  mockMarkPlayerReadyAction,
  mockSubmitBidAction,
  mockSelectTrumpAction,
  mockSubmitPlayAction,
  mockAddAiSeatAction,
  mockUpdateAiSeatAction,
  mockRemoveAiSeatAction,
  mockFetchAiRegistryAction,
} from '../../setupGameRoomActionsMock'
import {
  MockWebSocket,
  mockWebSocketInstances,
} from '@/test/setup/mock-websocket'
import { createStateForMock } from './game-room-client-helpers'

/**
 * Module-level state for tracking pending operations.
 * These should be reset in beforeEach hooks.
 */
export const gameRoomTestState = {
  initializedGameIds: new Set<number>(),
}

/**
 * Sets up fetch mock for /api/ws-token endpoint.
 * Call this in beforeEach hooks.
 *
 * @param originalFetch - The original fetch function to fallback to
 */
export function setupFetchMock(originalFetch: typeof global.fetch): void {
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
}

function deferred<T>(): { promise: Promise<T>; resolve: (value: T) => void } {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((r) => {
    resolve = r
  })
  return { promise, resolve }
}

/**
 * Creates a mock implementation for useMarkPlayerReady hook.
 *
 * @param options.trackPending - If true, tracks pending state to prevent concurrent calls
 * @param options.addDeferredDelay - If true, waits on a controlled promise before calling action (test resolves via resolveMarkPlayerReadyDelay)
 * @returns Object with mockUseMarkPlayerReady function and optional markPlayerReadyState, resolveMarkPlayerReadyDelay
 */
export function createMockUseMarkPlayerReady(options?: {
  trackPending?: boolean
  addDeferredDelay?: boolean
}) {
  const trackPending = options?.trackPending ?? false
  const addDeferredDelay = options?.addDeferredDelay ?? false

  // Track pending state if requested
  const markPlayerReadyState = trackPending ? { isPending: false } : undefined

  // Controlled promise for addDeferredDelay: test resolves when ready
  let currentDeferred: ReturnType<typeof deferred<void>> | null = null
  const resolveMarkPlayerReadyDelay = () => {
    currentDeferred?.resolve()
    currentDeferred = null
  }

  const mockUseMarkPlayerReady = vi.fn(() => {
    if (trackPending && markPlayerReadyState) {
      return {
        mutateAsync: async ({
          gameId,
          isReady,
        }: {
          gameId: number
          isReady: boolean
        }) => {
          if (markPlayerReadyState.isPending) {
            return // Don't call if already pending
          }
          markPlayerReadyState.isPending = true
          try {
            if (addDeferredDelay) {
              currentDeferred = deferred<void>()
              await currentDeferred.promise
            }
            const result = await mockMarkPlayerReadyAction(gameId, isReady)
            if (result.kind === 'error') {
              throw new Error(result.message)
            }
            return result
          } finally {
            markPlayerReadyState.isPending = false
          }
        },
        get isPending() {
          return markPlayerReadyState.isPending
        },
      }
    } else {
      return {
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
      }
    }
  })

  return {
    mockUseMarkPlayerReady,
    markPlayerReadyState, // Expose for tests that need to reset it
    resolveMarkPlayerReadyDelay: addDeferredDelay
      ? resolveMarkPlayerReadyDelay
      : undefined,
  }
}

/**
 * Creates standard mock implementations for all mutation hooks.
 * These hooks call the corresponding server actions.
 *
 * Use this pattern in test files:
 * ```ts
 * const { mockUseMarkPlayerReady, ... } = createMockMutationHooks()
 *
 * vi.mock('@/hooks/mutations/useGameRoomMutations', () => ({
 *   useMarkPlayerReady: () => mockUseMarkPlayerReady(),
 *   // ... etc
 * }))
 * ```
 */
export function createMockMutationHooks(options?: {
  trackMarkPlayerReadyPending?: boolean
  addMarkPlayerReadyDeferred?: boolean
}) {
  const {
    mockUseMarkPlayerReady,
    markPlayerReadyState,
    resolveMarkPlayerReadyDelay,
  } = createMockUseMarkPlayerReady({
    trackPending: options?.trackMarkPlayerReadyPending ?? false,
    addDeferredDelay: options?.addMarkPlayerReadyDeferred ?? false,
  })

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

  return {
    mockUseMarkPlayerReady,
    mockUseSubmitBid,
    mockUseSelectTrump,
    mockUseSubmitPlay,
    mockUseAddAiSeat,
    mockUseUpdateAiSeat,
    mockUseRemoveAiSeat,
    mockUseLeaveGame,
    markPlayerReadyState, // Expose for tests that need to reset it
    resolveMarkPlayerReadyDelay, // Expose when addMarkPlayerReadyDeferred is true
  }
}

/**
 * Resets all module-level test state.
 * Should be called in beforeEach hooks.
 */
export function resetGameRoomTestState() {
  gameRoomTestState.initializedGameIds.clear()
}

/**
 * Sets up all common mocks and state for GameRoomClient tests.
 * This includes:
 * - Clearing all mocks and using real timers
 * - Resetting module-level test state
 * - Resetting all action mocks
 * - Setting up environment variables
 * - Setting up WebSocket mock
 * - Setting up default mock implementations
 *
 * Call this at the start of beforeEach hooks, then call setupFetchMock() separately.
 *
 * Note: mockShowToast and mockHideToast are file-local mocks that are handled
 * by vi.clearAllMocks() - they don't need to be reset explicitly here.
 *
 * @example
 * ```ts
 * beforeEach(() => {
 *   setupGameRoomClientTest()
 *   setupFetchMock(originalFetch)
 *   // Test-specific setup here (e.g., mockFetchAiRegistryAction.mockClear())
 * })
 * ```
 */
export function setupGameRoomClientTest(): void {
  vi.clearAllMocks()
  vi.useRealTimers()
  resetGameRoomTestState()

  // Reset all action mocks to clear call history and implementation queues
  mockGetGameRoomStateAction.mockReset()
  mockMarkPlayerReadyAction.mockReset()
  mockSubmitBidAction.mockReset()
  mockSelectTrumpAction.mockReset()
  mockSubmitPlayAction.mockReset()
  mockAddAiSeatAction.mockReset()
  mockUpdateAiSeatAction.mockReset()
  mockRemoveAiSeatAction.mockReset()
  mockFetchAiRegistryAction.mockReset()

  // Reset WebSocket mock
  mockWebSocketInstances.length = 0
  vi.stubGlobal('WebSocket', MockWebSocket)

  // Set default mock implementations
  mockGetGameRoomStateAction.mockResolvedValue({
    kind: 'ok',
    data: createStateForMock(42),
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
    data: {
      entries: [
        { name: 'Tactician', version: '1.4.0' },
        { name: 'RandomPlayer', version: '1.0.0' },
      ],
      defaultName: 'Tactician',
    },
  })
}

/**
 * Tears down all common mocks and state for GameRoomClient tests.
 * This includes:
 * - Restoring real timers and clearing all timers
 * - Restoring original WebSocket and fetch globals
 * - Clearing environment variables
 *
 * Call this in afterEach hooks to ensure clean test isolation.
 *
 * @example
 * ```ts
 * afterEach(() => {
 *   teardownGameRoomClientTest()
 * })
 * ```
 */
export function teardownGameRoomClientTest(): void {
  vi.useRealTimers()
  vi.clearAllTimers()
  // Restore original WebSocket and fetch
  vi.unstubAllGlobals()
}
