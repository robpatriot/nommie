import { describe, expect, it, vi, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useGameRoomControls } from '@/app/game/[gameId]/_components/hooks/useGameRoomControls'
import type { PhaseSnapshot } from '@/lib/game-room/types'
import {
  biddingPhaseSnapshot,
  trickPhaseSnapshot,
  initPhaseSnapshot,
} from '../mocks/game-snapshot'

describe('useGameRoomControls', () => {
  const mockHandleSubmitBid = vi.fn().mockResolvedValue(undefined)
  const mockHandleSelectTrump = vi.fn().mockResolvedValue(undefined)
  const mockHandlePlayCard = vi.fn().mockResolvedValue(undefined)

  const defaultProps = {
    viewerSeatForInteractions: 0 as const,
    bidConstraints: { zeroBidLocked: false },
    handleSubmitBid: mockHandleSubmitBid,
    handleSelectTrump: mockHandleSelectTrump,
    handlePlayCard: mockHandlePlayCard,
    isBidPending: false,
    isTrumpPending: false,
    isPlayPending: false,
  }

  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('Bidding Controls', () => {
    it('returns bidding controls when in bidding phase and viewer has not bid', () => {
      const phase: PhaseSnapshot = {
        phase: 'Bidding',
        data: {
          ...biddingPhaseSnapshot,
          bids: [null, null, null, null] as [
            number | null,
            number | null,
            number | null,
            number | null,
          ],
        },
      }

      const { result } = renderHook(() =>
        useGameRoomControls({
          ...defaultProps,
          phase,
        })
      )

      expect(result.current.biddingControls).toBeDefined()
      expect(result.current.biddingControls?.viewerSeat).toBe(0)
      expect(result.current.biddingControls?.isPending).toBe(false)
      expect(result.current.biddingControls?.zeroBidLocked).toBe(false)
    })

    it('returns undefined when viewer has already bid', () => {
      const phase: PhaseSnapshot = {
        phase: 'Bidding',
        data: {
          ...biddingPhaseSnapshot,
          bids: [5, null, null, null] as [
            number | null,
            number | null,
            number | null,
            number | null,
          ],
        },
      }

      const { result } = renderHook(() =>
        useGameRoomControls({
          ...defaultProps,
          phase,
        })
      )

      expect(result.current.biddingControls).toBeUndefined()
    })

    it('returns undefined when not in bidding phase', () => {
      const phase = initPhaseSnapshot

      const { result } = renderHook(() =>
        useGameRoomControls({
          ...defaultProps,
          phase,
        })
      )

      expect(result.current.biddingControls).toBeUndefined()
    })

    it('returns undefined when viewerSeatForInteractions is null', () => {
      const phase: PhaseSnapshot = {
        phase: 'Bidding',
        data: {
          ...biddingPhaseSnapshot,
          bids: [null, null, null, null] as [
            number | null,
            number | null,
            number | null,
            number | null,
          ],
        },
      }

      const { result } = renderHook(() =>
        useGameRoomControls({
          ...defaultProps,
          phase,
          viewerSeatForInteractions: null,
        })
      )

      expect(result.current.biddingControls).toBeUndefined()
    })

    it('includes zeroBidLocked from bidConstraints', () => {
      const phase: PhaseSnapshot = {
        phase: 'Bidding',
        data: {
          ...biddingPhaseSnapshot,
          bids: [null, null, null, null] as [
            number | null,
            number | null,
            number | null,
            number | null,
          ],
        },
      }

      const { result } = renderHook(() =>
        useGameRoomControls({
          ...defaultProps,
          phase,
          bidConstraints: { zeroBidLocked: true },
        })
      )

      expect(result.current.biddingControls?.zeroBidLocked).toBe(true)
    })

    it('reflects isBidPending state', () => {
      const phase: PhaseSnapshot = {
        phase: 'Bidding',
        data: {
          ...biddingPhaseSnapshot,
          bids: [null, null, null, null] as [
            number | null,
            number | null,
            number | null,
            number | null,
          ],
        },
      }

      const { result } = renderHook(() =>
        useGameRoomControls({
          ...defaultProps,
          phase,
          isBidPending: true,
        })
      )

      expect(result.current.biddingControls?.isPending).toBe(true)
    })
  })

  describe('Trump Controls', () => {
    it('returns trump controls when in trump select phase', () => {
      const phase: PhaseSnapshot = {
        phase: 'TrumpSelect',
        data: {
          round: {
            hand_size: 8,
            leader: 0,
            bid_winner: 0,
            trump: null,
            tricks_won: [0, 0, 0, 0],
            bids: [2, 3, 4, 5] as [
              number | null,
              number | null,
              number | null,
              number | null,
            ],
          },
          to_act: 0,
          allowed_trumps: ['HEARTS', 'SPADES'],
          last_trick: null,
        },
      }

      const { result } = renderHook(() =>
        useGameRoomControls({
          ...defaultProps,
          phase,
        })
      )

      expect(result.current.trumpControls).toBeDefined()
      expect(result.current.trumpControls?.viewerSeat).toBe(0)
      expect(result.current.trumpControls?.toAct).toBe(0)
      expect(result.current.trumpControls?.canSelect).toBe(true)
      expect(result.current.trumpControls?.allowedTrumps).toEqual([
        'HEARTS',
        'SPADES',
      ])
    })

    it('sets canSelect to false when not viewer turn', () => {
      const phase: PhaseSnapshot = {
        phase: 'TrumpSelect',
        data: {
          round: {
            hand_size: 8,
            leader: 0,
            bid_winner: 0,
            trump: null,
            tricks_won: [0, 0, 0, 0],
            bids: [2, 3, 4, 5] as [
              number | null,
              number | null,
              number | null,
              number | null,
            ],
          },
          to_act: 1,
          allowed_trumps: ['HEARTS', 'SPADES'],
          last_trick: null,
        },
      }

      const { result } = renderHook(() =>
        useGameRoomControls({
          ...defaultProps,
          phase,
        })
      )

      expect(result.current.trumpControls?.canSelect).toBe(false)
      expect(result.current.trumpControls?.onSelect).toBeUndefined()
    })

    it('returns undefined when not in trump select phase', () => {
      const phase = initPhaseSnapshot

      const { result } = renderHook(() =>
        useGameRoomControls({
          ...defaultProps,
          phase,
        })
      )

      expect(result.current.trumpControls).toBeUndefined()
    })

    it('returns undefined when viewerSeatForInteractions is null', () => {
      const phase: PhaseSnapshot = {
        phase: 'TrumpSelect',
        data: {
          round: {
            hand_size: 8,
            leader: 0,
            bid_winner: 0,
            trump: null,
            tricks_won: [0, 0, 0, 0],
            bids: [2, 3, 4, 5] as [
              number | null,
              number | null,
              number | null,
              number | null,
            ],
          },
          to_act: 0,
          allowed_trumps: ['HEARTS'],
          last_trick: null,
        },
      }

      const { result } = renderHook(() =>
        useGameRoomControls({
          ...defaultProps,
          phase,
          viewerSeatForInteractions: null,
        })
      )

      expect(result.current.trumpControls).toBeUndefined()
    })

    it('reflects isTrumpPending state', () => {
      const phase: PhaseSnapshot = {
        phase: 'TrumpSelect',
        data: {
          round: {
            hand_size: 8,
            leader: 0,
            bid_winner: 0,
            trump: null,
            tricks_won: [0, 0, 0, 0],
            bids: [2, 3, 4, 5] as [
              number | null,
              number | null,
              number | null,
              number | null,
            ],
          },
          to_act: 0,
          allowed_trumps: ['HEARTS'],
          last_trick: null,
        },
      }

      const { result } = renderHook(() =>
        useGameRoomControls({
          ...defaultProps,
          phase,
          isTrumpPending: true,
        })
      )

      expect(result.current.trumpControls?.isPending).toBe(true)
    })
  })

  describe('Play Controls', () => {
    it('returns play controls when in trick phase', () => {
      const phase: PhaseSnapshot = {
        phase: 'Trick',
        data: {
          ...trickPhaseSnapshot,
          playable: ['2H', '3C', '5S'],
        },
      }

      const { result } = renderHook(() =>
        useGameRoomControls({
          ...defaultProps,
          phase,
        })
      )

      expect(result.current.playControls).toBeDefined()
      expect(result.current.playControls?.viewerSeat).toBe(0)
      expect(result.current.playControls?.playable).toEqual(['2H', '3C', '5S'])
      expect(result.current.playControls?.isPending).toBe(false)
    })

    it('returns undefined when not in trick phase', () => {
      const phase = initPhaseSnapshot

      const { result } = renderHook(() =>
        useGameRoomControls({
          ...defaultProps,
          phase,
        })
      )

      expect(result.current.playControls).toBeUndefined()
    })

    it('returns undefined when viewerSeatForInteractions is null', () => {
      const phase: PhaseSnapshot = {
        phase: 'Trick',
        data: {
          ...trickPhaseSnapshot,
          playable: ['2H'],
        },
      }

      const { result } = renderHook(() =>
        useGameRoomControls({
          ...defaultProps,
          phase,
          viewerSeatForInteractions: null,
        })
      )

      expect(result.current.playControls).toBeUndefined()
    })

    it('reflects isPlayPending state', () => {
      const phase: PhaseSnapshot = {
        phase: 'Trick',
        data: {
          ...trickPhaseSnapshot,
          playable: ['2H'],
        },
      }

      const { result } = renderHook(() =>
        useGameRoomControls({
          ...defaultProps,
          phase,
          isPlayPending: true,
        })
      )

      expect(result.current.playControls?.isPending).toBe(true)
    })
  })
})
