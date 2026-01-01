import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, waitFor, act, createTestQueryClient } from '../utils'
import userEvent from '@testing-library/user-event'
import type { ReactNode } from 'react'
import { queryKeys } from '@/lib/queries/query-keys'

import { GameRoomClient } from '@/app/game/[gameId]/_components/game-room-client'
import { biddingSnapshotFixture } from '../mocks/game-snapshot'
import {
  mockGetGameRoomSnapshotAction,
  mockSubmitBidAction,
  mockFetchAiRegistryAction,
} from '../../setupGameRoomActionsMock'
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
    // Clear AI registry calls before each test
    mockFetchAiRegistryAction.mockClear()
  })

  afterEach(() => {
    teardownGameRoomClientTest()
  })

  describe('Bid action', () => {
    it('submits bid', async () => {
      // Create a bidding snapshot where viewer (seat 0) hasn't bid yet
      // biddingSnapshotFixture.phase is 'Bidding' phase, so we can safely assert the type
      if (biddingSnapshotFixture.phase.phase !== 'Bidding') {
        throw new Error('Expected biddingSnapshotFixture to have Bidding phase')
      }
      const biddingPhase = biddingSnapshotFixture.phase
      const biddingSnapshotWithNoBid = {
        ...biddingSnapshotFixture,
        phase: {
          ...biddingPhase,
          data: {
            ...biddingPhase.data,
            bids: [null, null, null, null] as [
              number | null,
              number | null,
              number | null,
              number | null,
            ], // Viewer hasn't bid yet
            to_act: 0, // It's the viewer's turn
          },
        },
      } as typeof biddingSnapshotFixture
      const biddingData = createInitialData(biddingSnapshotWithNoBid, {
        viewerSeat: 0,
        viewerHand: ['2H', '3C'],
        version: 1,
      })

      // Initialize query cache with bidding data BEFORE rendering
      const queryClient = createTestQueryClient()
      queryClient.setQueryData(queryKeys.games.snapshot(42), biddingData)

      // Override the default mock to return bidding data for this test
      // This ensures if the query fetches, it returns the correct data
      mockGetGameRoomSnapshotAction.mockResolvedValueOnce({
        kind: 'ok',
        data: biddingData,
      })

      await act(async () => {
        render(<GameRoomClient initialData={biddingData} gameId={42} />, {
          queryClient,
        })
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
            version: 1,
          })
        },
        { timeout: 2000 }
      )
    })
  })
})
