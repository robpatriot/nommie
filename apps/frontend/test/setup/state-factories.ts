import { vi } from 'vitest'
import type { Seat, Trump } from '@/lib/game-room/types'

/**
 * Factory for creating bidding state (used by BiddingPanel)
 */
export function createBiddingState(overrides?: {
  viewerSeat?: Seat
  isPending?: boolean
  zeroBidLocked?: boolean
  onSubmit?: (bid: number) => Promise<void>
}) {
  return {
    viewerSeat: (overrides?.viewerSeat ?? 0) as Seat,
    isPending: overrides?.isPending ?? false,
    zeroBidLocked: overrides?.zeroBidLocked ?? false,
    onSubmit: overrides?.onSubmit ?? vi.fn().mockResolvedValue(undefined),
  }
}

/**
 * Factory for creating trump selection state (used by BiddingPanel)
 */
export function createTrumpState(overrides?: {
  viewerSeat?: Seat
  toAct?: Seat
  allowedTrumps?: Trump[]
  canSelect?: boolean
  isPending?: boolean
  onSelect?: (trump: Trump) => Promise<void>
}) {
  return {
    viewerSeat: (overrides?.viewerSeat ?? 0) as Seat,
    toAct: (overrides?.toAct ?? 0) as Seat,
    allowedTrumps: overrides?.allowedTrumps ?? [
      'CLUBS',
      'DIAMONDS',
      'HEARTS',
      'SPADES',
      'NO_TRUMPS',
    ],
    canSelect: overrides?.canSelect ?? true,
    isPending: overrides?.isPending ?? false,
    onSelect: overrides?.onSelect ?? vi.fn().mockResolvedValue(undefined),
  }
}

/**
 * Factory for creating play state (used by PlayPanel)
 */
export function createPlayState(overrides?: {
  viewerSeat?: Seat
  playable?: string[]
  isPending?: boolean
  onPlay?: (card: string) => Promise<void>
}) {
  return {
    viewerSeat: (overrides?.viewerSeat ?? 0) as Seat,
    playable: overrides?.playable ?? ['2H', '3C', '5S'],
    isPending: overrides?.isPending ?? false,
    onPlay: overrides?.onPlay ?? vi.fn().mockResolvedValue(undefined),
  }
}
