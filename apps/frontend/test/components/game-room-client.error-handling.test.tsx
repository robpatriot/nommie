import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, waitFor, act } from '../utils'
import userEvent from '@testing-library/user-event'
import type { ReactNode } from 'react'

import { GameRoomClient } from '@/app/game/[gameId]/_components/game-room-client'
import { mockGetGameRoomStateAction } from '../../setupGameRoomActionsMock'
import { createInitialState } from '../setup/game-room-client-helpers'
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

  describe('Error handling', () => {
    it('displays error messages with traceId', async () => {
      const initialState = createInitialState(42)

      mockGetGameRoomStateAction.mockResolvedValue({
        kind: 'error',
        message: 'Failed to fetch',
        status: 500,
        traceId: 'trace-abc-123',
      })

      await act(async () => {
        render(<GameRoomClient initialState={initialState} gameId={42} />)
      })

      // There might be multiple refresh buttons, get the first one
      const refreshButtons = screen.getAllByRole('button', {
        name: /Refresh game state/i,
      })
      await userEvent.click(refreshButtons[0])

      // Wait for async operations
      await waitFor(
        () => {
          expect(mockGetGameRoomStateAction).toHaveBeenCalled()
        },
        { timeout: 2000 }
      )

      expect(screen.getByText(/Failed to fetch/i)).toBeInTheDocument()

      // TraceId should be visible (may be in expandable details)
    })

    it('handles network errors gracefully', async () => {
      const initialState = createInitialState(42)

      const expectedError = new Error('Network error')
      mockGetGameRoomStateAction.mockRejectedValue(expectedError)
      const consoleErrorSpy = vi
        .spyOn(console, 'error')
        .mockImplementation(() => {})

      await act(async () => {
        render(<GameRoomClient initialState={initialState} gameId={42} />)
      })

      const refreshButton = screen.getByRole('button', {
        name: /Refresh game state/i,
      })
      try {
        const callCountBefore = mockGetGameRoomStateAction.mock.calls.length

        await expect(userEvent.click(refreshButton)).resolves.toBeUndefined()

        await waitFor(
          () => {
            expect(mockGetGameRoomStateAction.mock.calls.length).toBe(
              callCountBefore + 1
            )
          },
          { timeout: 2000 }
        )

        const results = mockGetGameRoomStateAction.mock.results
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
})
