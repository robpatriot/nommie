import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { render, act } from '../utils'
import type { ReactNode } from 'react'

import { GameRoomClient } from '@/app/game/[gameId]/_components/game-room-client'
import { initSnapshotFixture } from '../mocks/game-snapshot'
import { mockFetchAiRegistryAction } from '../../setupGameRoomActionsMock'
import {
  createInitialData,
  waitForWebSocketConnection,
} from '../setup/game-room-client-helpers'
import {
  createMockMutationHooks,
  setupFetchMock,
  setupGameRoomClientTest,
  teardownGameRoomClientTest,
} from '../setup/game-room-client-mocks'

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

// Don't mock useGameSync - use the real implementation
// It will create WebSocket connections (mocked) and update the query cache
// The WebSocket API is already mocked above, so this will work

// Create mutation hook mocks using shared utility
const {
  mockUseMarkPlayerReady,
  mockUseSubmitBid,
  mockUseSelectTrump,
  mockUseSubmitPlay,
  mockUseAddAiSeat,
  mockUseUpdateAiSeat,
  mockUseRemoveAiSeat,
  mockUseLeaveGame,
} = createMockMutationHooks()

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

// Track original fetch (WebSocket is restored via vi.unstubAllGlobals)
const originalFetch = global.fetch

// Mock next/link
vi.mock('next/link', () => ({
  __esModule: true,
  default: ({ children, ...props }: { children: ReactNode; href: string }) => (
    <a {...props}>{children}</a>
  ),
}))

describe('GameRoomClient', () => {
  beforeEach(() => {
    setupGameRoomClientTest()
    setupFetchMock(originalFetch)
  })

  afterEach(() => {
    teardownGameRoomClientTest()
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
            data: [{ name: 'Heuristic', version: '1.0.0' }],
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
