import type { GameRoomState } from './types'
import type { BidConstraints, GameSnapshot, Seat } from '@/lib/game-room/types'
import { extractPlayerNames } from '@/utils/player-names'
import { isValidSeat } from '@/utils/seat-validation'

export function selectSnapshot(state: GameRoomState): GameSnapshot {
  return state.game
}

export function selectViewerSeat(state: GameRoomState): Seat | null {
  const seat = state.viewer.seat
  return typeof seat === 'number' && isValidSeat(seat) ? (seat as Seat) : null
}

export function selectViewerHand(state: GameRoomState): string[] {
  const hand = state.viewer.hand
  return Array.isArray(hand) && hand.every((c) => typeof c === 'string')
    ? hand
    : []
}

export function selectBidConstraints(
  state: GameRoomState
): BidConstraints | null {
  const bc = state.viewer.bidConstraints
  return bc ? { zeroBidLocked: Boolean(bc.zeroBidLocked) } : null
}

export function selectPlayerNames(
  state: GameRoomState
): [string, string, string, string] {
  return extractPlayerNames(state.game.game.seating)
}

export function selectVersion(state: GameRoomState): number {
  return state.version
}

export function selectHostSeat(state: GameRoomState): Seat | null {
  const raw = state.game.game.host_seat
  return typeof raw === 'number' && isValidSeat(raw) ? (raw as Seat) : null
}
