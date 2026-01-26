import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, waitFor, act } from '../utils'
import userEvent from '@testing-library/user-event'
import type { ReactNode } from 'react'

import { GameRoomClient } from '@/app/game/[gameId]/_components/game-room-client'
import {
  mockGetGameRoomSnapshotAction,
  mockMarkPlayerReadyAction,
} from '../../setupGameRoomActionsMock'
import { createInitialDataWithVersion } from '../setup/game-room-client-helpers'
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

// Create mutation hook mocks with pending state tracking for action coordination tests
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
} = createMockMutationHooks({ trackMarkPlayerReadyPending: true })

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

  describe('Action coordination', () => {
    it('prevents actions when another action is in progress', async () => {
      const initialData = createInitialDataWithVersion(42, 1)

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
      const initialData = createInitialDataWithVersion(42, 1)

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
