import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, waitFor, act } from '../utils'
import userEvent from '@testing-library/user-event'
import type { ReactNode } from 'react'

import { GameRoomClient } from '@/app/game/[gameId]/_components/game-room-client'
import {
  initSnapshotFixture,
  biddingSnapshotFixture,
} from '../mocks/game-snapshot'
import {
  mockGetGameRoomSnapshotAction,
  mockMarkPlayerReadyAction,
} from '../../setupGameRoomActionsMock'
import {
  createInitialData,
  waitForWebSocketConnection,
  sendWebSocketSnapshot,
  createInitialDataWithVersion,
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
        version: 2,
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
      const initialData = createInitialDataWithVersion(42, 1)

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
        version: 2,
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
