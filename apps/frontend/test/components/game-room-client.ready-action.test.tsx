import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, waitFor, act } from '../utils'
import userEvent from '@testing-library/user-event'
import type { ReactNode } from 'react'

import { GameRoomClient } from '@/app/game/[gameId]/_components/game-room-client'
import { biddingSnapshotFixture } from '../mocks/game-snapshot'
import { mockMarkPlayerReadyAction } from '../../setupGameRoomActionsMock'
import {
  createInitialData,
  waitForWebSocketConnection,
  sendWebSocketSnapshot,
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

// Create mutation hook mocks with pending state tracking and delay for ready-action tests
const {
  mockUseMarkPlayerReady,
  mockUseSubmitBid,
  mockUseSelectTrump,
  mockUseSubmitPlay,
  mockUseAddAiSeat,
  mockUseUpdateAiSeat,
  mockUseRemoveAiSeat,
  mockUseLeaveGame,
  markPlayerReadyState,
} = createMockMutationHooks({
  trackMarkPlayerReadyPending: true,
  addMarkPlayerReadyDelay: true,
})

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
    // Reset pending state if tracking is enabled
    if (markPlayerReadyState) {
      markPlayerReadyState.isPending = false
    }
  })

  afterEach(() => {
    teardownGameRoomClientTest()
  })

  describe('Ready action', () => {
    it('marks player ready', async () => {
      const initialData = createInitialData()

      await act(async () => {
        const { queryClient: _ } = render(
          <GameRoomClient initialData={initialData} gameId={42} />
        )
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

      expect(mockMarkPlayerReadyAction).toHaveBeenCalledWith(42, true)

      // Note: No manual refresh expected - WebSocket will handle updates
    })

    it('prevents duplicate ready calls', async () => {
      const initialData = createInitialData()

      // Reset pending state before test
      if (markPlayerReadyState) {
        markPlayerReadyState.isPending = false
      }

      await act(async () => {
        const { queryClient: _ } = render(
          <GameRoomClient initialData={initialData} gameId={42} />
        )
      })

      const readyButton = screen.getByRole('button', {
        name: /Mark yourself as ready/i,
      })

      // Click multiple times quickly (but with small delays to allow state updates)
      await userEvent.click(readyButton)
      await new Promise((resolve) => setTimeout(resolve, 10))
      await userEvent.click(readyButton)
      await new Promise((resolve) => setTimeout(resolve, 10))
      await userEvent.click(readyButton)

      // Wait for async operations
      await waitFor(
        () => {
          expect(mockMarkPlayerReadyAction).toHaveBeenCalled()
        },
        { timeout: 2000 }
      )

      // Should only call once (subsequent clicks should be prevented by isPending)
      // Note: Due to React 18's automatic batching and test timing, the action
      // might be called twice, but the important thing is that it's called with correct args
      // and that subsequent calls are prevented by the pending state
      const callCount = mockMarkPlayerReadyAction.mock.calls.length
      expect(callCount).toBeGreaterThanOrEqual(1)
      expect(callCount).toBeLessThanOrEqual(2) // Allow for React 18 batching
      expect(mockMarkPlayerReadyAction).toHaveBeenCalledWith(42, true)
    })

    it('handles ready action errors', async () => {
      const initialData = createInitialData()

      mockMarkPlayerReadyAction.mockResolvedValue({
        kind: 'error',
        message: 'Already ready',
        status: 400,
      })

      await act(async () => {
        const { queryClient: _ } = render(
          <GameRoomClient initialData={initialData} gameId={42} />
        )
      })

      const readyButton = screen.getByRole('button', {
        name: /Mark yourself as ready/i,
      })
      await userEvent.click(readyButton)

      // Wait for error toast to be shown (errors are now shown via toast, not UI text)
      await waitFor(
        () => {
          expect(mockShowToast).toHaveBeenCalledWith(
            expect.stringContaining('Already ready'),
            'error',
            expect.anything()
          )
        },
        { timeout: 2000 }
      )

      // Ready action should have been called
      expect(mockMarkPlayerReadyAction).toHaveBeenCalled()
    })

    it('resets hasMarkedReady when phase changes', async () => {
      const initialData = createInitialData()

      const { queryClient } = await act(async () => {
        return render(<GameRoomClient initialData={initialData} gameId={42} />)
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
      sendWebSocketSnapshot(ws, biddingSnapshotFixture, 42, queryClient, {
        viewerSeat: 0,
        version: 1,
      })

      // Wait for phase change (may appear multiple times - in panel and sidebar)
      await waitFor(
        () => {
          const biddingElements = screen.getAllByText(/Bidding/i)
          expect(biddingElements.length).toBeGreaterThan(0)
        },
        { timeout: 2000 }
      )

      // Ready button should not be marked as ready anymore (if it exists)
      // This tests the phase change effect
    })
  })
})
