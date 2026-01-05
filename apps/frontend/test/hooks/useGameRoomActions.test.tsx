import { describe, expect, it, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { QueryClientProvider } from '@tanstack/react-query'
import type { QueryClient } from '@tanstack/react-query'
import React, { type ReactNode } from 'react'
import { useGameRoomActions } from '@/app/game/[gameId]/_components/hooks/useGameRoomActions'
import { createTestQueryClient } from '../utils'
import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import { initSnapshotFixture } from '../mocks/game-snapshot'
import { queryKeys } from '@/lib/queries/query-keys'
import {
  createBiddingPhase,
  createTrumpPhase,
  createTrickPhase,
} from '../setup/phase-factories'
import type { PhaseSnapshot } from '@/lib/game-room/types'

// Mock mutations
const mockMutateAsync = vi.fn()
const mockUseMarkPlayerReady = vi.fn(() => ({
  mutateAsync: mockMutateAsync,
  isPending: false,
}))
const mockUseLeaveGame = vi.fn(() => ({
  mutateAsync: mockMutateAsync,
  isPending: false,
}))
const mockUseSubmitBid = vi.fn(() => ({
  mutateAsync: mockMutateAsync,
  isPending: false,
}))
const mockUseSelectTrump = vi.fn(() => ({
  mutateAsync: mockMutateAsync,
  isPending: false,
}))
const mockUseSubmitPlay = vi.fn(() => ({
  mutateAsync: mockMutateAsync,
  isPending: false,
}))

vi.mock('@/hooks/mutations/useGameRoomMutations', () => ({
  useMarkPlayerReady: () => mockUseMarkPlayerReady(),
  useLeaveGame: () => mockUseLeaveGame(),
  useSubmitBid: () => mockUseSubmitBid(),
  useSelectTrump: () => mockUseSelectTrump(),
  useSubmitPlay: () => mockUseSubmitPlay(),
}))

// Mock router
const mockPush = vi.fn()
vi.mock('next/navigation', () => ({
  useRouter: () => ({
    push: mockPush,
  }),
}))

// Mock window.confirm
const mockConfirm = vi.fn()

function createWrapper(queryClient: QueryClient) {
  const Wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  )
  Wrapper.displayName = 'TestQueryClientProvider'
  return Wrapper
}

function createSnapshotData(
  overrides?: Partial<GameRoomSnapshotPayload>
): GameRoomSnapshotPayload {
  const version = overrides?.version ?? 1
  return {
    snapshot: initSnapshotFixture,
    etag: `"game-42-v${version}"`,
    version,
    playerNames: ['Alex', 'Bailey', 'Casey', 'Dakota'],
    viewerSeat: 0,
    viewerHand: [],
    timestamp: new Date().toISOString(),
    hostSeat: 0,
    bidConstraints: null,
    ...overrides,
  }
}

describe('useGameRoomActions', () => {
  let queryClient: QueryClient
  const mockShowToast = vi.fn()
  const mockDisconnect = vi.fn()
  const mockConnect = vi.fn().mockResolvedValue(undefined)

  beforeEach(() => {
    queryClient = createTestQueryClient()
    vi.clearAllMocks()
    mockMutateAsync.mockResolvedValue(undefined)
    mockConfirm.mockReturnValue(true)
    window.confirm = mockConfirm

    // Set up initial cache data
    queryClient.setQueryData(
      queryKeys.games.snapshot(42),
      createSnapshotData({ version: 1 })
    )
  })

  describe('markReady', () => {
    it('marks player ready successfully', async () => {
      const { result } = renderHook(
        () =>
          useGameRoomActions({
            gameId: 42,
            canMarkReady: true,
            hasMarkedReady: false,
            setHasMarkedReady: vi.fn(),
            showToast: mockShowToast,
            disconnect: mockDisconnect,
            connect: mockConnect,
            phase: { phase: 'Init' },
            viewerSeat: 0,
          }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await act(async () => {
        await result.current.markReady()
      })

      expect(mockMutateAsync).toHaveBeenCalledWith({
        gameId: 42,
        isReady: true,
      })
    })

    it('does not mark ready when canMarkReady is false', async () => {
      const { result } = renderHook(
        () =>
          useGameRoomActions({
            gameId: 42,
            canMarkReady: false,
            hasMarkedReady: false,
            setHasMarkedReady: vi.fn(),
            showToast: mockShowToast,
            disconnect: mockDisconnect,
            connect: mockConnect,
            phase: {
              phase: 'Bidding',
              data: createBiddingPhase(),
            } as PhaseSnapshot,
            viewerSeat: 0,
          }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await act(async () => {
        await result.current.markReady()
      })

      expect(mockMutateAsync).not.toHaveBeenCalled()
    })

    it('handles mark ready error', async () => {
      const error = new Error('Failed to mark ready')
      mockMutateAsync.mockRejectedValueOnce(error)

      const { result } = renderHook(
        () =>
          useGameRoomActions({
            gameId: 42,
            canMarkReady: true,
            hasMarkedReady: false,
            setHasMarkedReady: vi.fn(),
            showToast: mockShowToast,
            disconnect: mockDisconnect,
            connect: mockConnect,
            phase: { phase: 'Init' },
            viewerSeat: 0,
          }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await act(async () => {
        await result.current.markReady()
      })

      expect(mockShowToast).toHaveBeenCalledWith(
        expect.stringContaining('Failed'),
        'error',
        expect.any(Object)
      )
    })
  })

  describe('handleSubmitBid', () => {
    it('submits bid successfully', async () => {
      const { result } = renderHook(
        () =>
          useGameRoomActions({
            gameId: 42,
            canMarkReady: false,
            hasMarkedReady: false,
            setHasMarkedReady: vi.fn(),
            showToast: mockShowToast,
            disconnect: mockDisconnect,
            connect: mockConnect,
            phase: {
              phase: 'Bidding',
              data: createBiddingPhase(),
            } as PhaseSnapshot,
            viewerSeat: 0,
          }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await act(async () => {
        await result.current.handleSubmitBid(5)
      })

      expect(mockMutateAsync).toHaveBeenCalledWith({
        gameId: 42,
        bid: 5,
        version: 1,
      })
      expect(mockShowToast).toHaveBeenCalledWith(expect.any(String), 'success')
    })

    it('shows error when version is missing', async () => {
      queryClient.setQueryData(queryKeys.games.snapshot(42), null)

      const { result } = renderHook(
        () =>
          useGameRoomActions({
            gameId: 42,
            canMarkReady: false,
            hasMarkedReady: false,
            setHasMarkedReady: vi.fn(),
            showToast: mockShowToast,
            disconnect: mockDisconnect,
            connect: mockConnect,
            phase: {
              phase: 'Bidding',
              data: createBiddingPhase(),
            } as PhaseSnapshot,
            viewerSeat: 0,
          }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await act(async () => {
        await result.current.handleSubmitBid(5)
      })

      expect(mockMutateAsync).not.toHaveBeenCalled()
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.stringContaining('version'),
        'error'
      )
    })

    it('handles bid submission error', async () => {
      const error = new Error('Failed to submit bid')
      mockMutateAsync.mockRejectedValueOnce(error)

      const { result } = renderHook(
        () =>
          useGameRoomActions({
            gameId: 42,
            canMarkReady: false,
            hasMarkedReady: false,
            setHasMarkedReady: vi.fn(),
            showToast: mockShowToast,
            disconnect: mockDisconnect,
            connect: mockConnect,
            phase: {
              phase: 'Bidding',
              data: createBiddingPhase(),
            } as PhaseSnapshot,
            viewerSeat: 0,
          }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await act(async () => {
        await result.current.handleSubmitBid(5)
      })

      expect(mockShowToast).toHaveBeenCalledWith(
        expect.stringContaining('Failed'),
        'error',
        expect.any(Object)
      )
    })
  })

  describe('handleSelectTrump', () => {
    it('submits trump selection successfully', async () => {
      const { result } = renderHook(
        () =>
          useGameRoomActions({
            gameId: 42,
            canMarkReady: false,
            hasMarkedReady: false,
            setHasMarkedReady: vi.fn(),
            showToast: mockShowToast,
            disconnect: mockDisconnect,
            connect: mockConnect,
            phase: {
              phase: 'TrumpSelect',
              data: createTrumpPhase(),
            } as PhaseSnapshot,
            viewerSeat: 0,
          }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await act(async () => {
        await result.current.handleSelectTrump('HEARTS')
      })

      expect(mockMutateAsync).toHaveBeenCalledWith({
        gameId: 42,
        trump: 'HEARTS',
        version: 1,
      })
      expect(mockShowToast).toHaveBeenCalledWith(expect.any(String), 'success')
    })

    it('handles trump selection error', async () => {
      const error = new Error('Failed to select trump')
      mockMutateAsync.mockRejectedValueOnce(error)

      const { result } = renderHook(
        () =>
          useGameRoomActions({
            gameId: 42,
            canMarkReady: false,
            hasMarkedReady: false,
            setHasMarkedReady: vi.fn(),
            showToast: mockShowToast,
            disconnect: mockDisconnect,
            connect: mockConnect,
            phase: {
              phase: 'TrumpSelect',
              data: createTrumpPhase(),
            } as PhaseSnapshot,
            viewerSeat: 0,
          }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await act(async () => {
        await result.current.handleSelectTrump('HEARTS')
      })

      expect(mockShowToast).toHaveBeenCalledWith(
        expect.stringContaining('Failed'),
        'error',
        expect.any(Object)
      )
    })
  })

  describe('handlePlayCard', () => {
    it('submits card play successfully', async () => {
      const { result } = renderHook(
        () =>
          useGameRoomActions({
            gameId: 42,
            canMarkReady: false,
            hasMarkedReady: false,
            setHasMarkedReady: vi.fn(),
            showToast: mockShowToast,
            disconnect: mockDisconnect,
            connect: mockConnect,
            phase: {
              phase: 'Trick',
              data: createTrickPhase(),
            } as PhaseSnapshot,
            viewerSeat: 0,
          }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await act(async () => {
        await result.current.handlePlayCard('2H')
      })

      expect(mockMutateAsync).toHaveBeenCalledWith({
        gameId: 42,
        card: '2H',
        version: 1,
      })
      expect(mockShowToast).toHaveBeenCalledWith(expect.any(String), 'success')
    })

    it('handles card play error', async () => {
      const error = new Error('Failed to play card')
      mockMutateAsync.mockRejectedValueOnce(error)

      const { result } = renderHook(
        () =>
          useGameRoomActions({
            gameId: 42,
            canMarkReady: false,
            hasMarkedReady: false,
            setHasMarkedReady: vi.fn(),
            showToast: mockShowToast,
            disconnect: mockDisconnect,
            connect: mockConnect,
            phase: {
              phase: 'Trick',
              data: createTrickPhase(),
            } as PhaseSnapshot,
            viewerSeat: 0,
          }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await act(async () => {
        await result.current.handlePlayCard('2H')
      })

      expect(mockShowToast).toHaveBeenCalledWith(
        expect.stringContaining('Failed'),
        'error',
        expect.any(Object)
      )
    })
  })

  describe('handleLeaveGame', () => {
    it('leaves game successfully when not active', async () => {
      const { result } = renderHook(
        () =>
          useGameRoomActions({
            gameId: 42,
            canMarkReady: false,
            hasMarkedReady: false,
            setHasMarkedReady: vi.fn(),
            showToast: mockShowToast,
            disconnect: mockDisconnect,
            connect: mockConnect,
            phase: { phase: 'Init' },
            viewerSeat: 0,
          }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await act(async () => {
        await result.current.handleLeaveGame()
      })

      expect(mockDisconnect).toHaveBeenCalled()
      expect(mockMutateAsync).toHaveBeenCalledWith({
        gameId: 42,
        version: 1,
      })
      expect(mockPush).toHaveBeenCalledWith('/lobby')
      expect(mockShowToast).toHaveBeenCalledWith(expect.any(String), 'success')
    })

    it('shows confirmation dialog for active game', async () => {
      mockConfirm.mockReturnValueOnce(true)

      const { result } = renderHook(
        () =>
          useGameRoomActions({
            gameId: 42,
            canMarkReady: false,
            hasMarkedReady: false,
            setHasMarkedReady: vi.fn(),
            showToast: mockShowToast,
            disconnect: mockDisconnect,
            connect: mockConnect,
            phase: {
              phase: 'Bidding',
              data: createBiddingPhase(),
            } as PhaseSnapshot,
            viewerSeat: 0,
          }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await act(async () => {
        await result.current.handleLeaveGame()
      })

      expect(mockConfirm).toHaveBeenCalled()
      expect(mockDisconnect).toHaveBeenCalled()
      expect(mockMutateAsync).toHaveBeenCalled()
    })

    it('does not leave when confirmation is cancelled', async () => {
      mockConfirm.mockReturnValueOnce(false)

      const { result } = renderHook(
        () =>
          useGameRoomActions({
            gameId: 42,
            canMarkReady: false,
            hasMarkedReady: false,
            setHasMarkedReady: vi.fn(),
            showToast: mockShowToast,
            disconnect: mockDisconnect,
            connect: mockConnect,
            phase: {
              phase: 'Bidding',
              data: createBiddingPhase(),
            } as PhaseSnapshot,
            viewerSeat: 0,
          }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await act(async () => {
        await result.current.handleLeaveGame()
      })

      expect(mockConfirm).toHaveBeenCalled()
      expect(mockDisconnect).not.toHaveBeenCalled()
      expect(mockMutateAsync).not.toHaveBeenCalled()
    })

    it('reconnects when leave fails', async () => {
      const error = new Error('Failed to leave')
      mockMutateAsync.mockRejectedValueOnce(error)

      const { result } = renderHook(
        () =>
          useGameRoomActions({
            gameId: 42,
            canMarkReady: false,
            hasMarkedReady: false,
            setHasMarkedReady: vi.fn(),
            showToast: mockShowToast,
            disconnect: mockDisconnect,
            connect: mockConnect,
            phase: { phase: 'Init' },
            viewerSeat: 0,
          }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      await act(async () => {
        await result.current.handleLeaveGame()
      })

      expect(mockConnect).toHaveBeenCalled()
      expect(mockPush).not.toHaveBeenCalled()
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.stringContaining('Failed'),
        'error',
        expect.any(Object)
      )
    })
  })

  describe('Pending States', () => {
    it('returns pending states from mutations', () => {
      mockUseSubmitBid.mockReturnValueOnce({
        mutateAsync: mockMutateAsync,
        isPending: true,
      })

      const { result } = renderHook(
        () =>
          useGameRoomActions({
            gameId: 42,
            canMarkReady: false,
            hasMarkedReady: false,
            setHasMarkedReady: vi.fn(),
            showToast: mockShowToast,
            disconnect: mockDisconnect,
            connect: mockConnect,
            phase: {
              phase: 'Bidding',
              data: createBiddingPhase(),
            } as PhaseSnapshot,
            viewerSeat: 0,
          }),
        {
          wrapper: createWrapper(queryClient),
        }
      )

      expect(result.current.isBidPending).toBe(true)
    })
  })
})
