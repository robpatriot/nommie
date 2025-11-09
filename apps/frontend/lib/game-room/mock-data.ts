import type { Card, GameSnapshot, Seat, SeatPublic } from './types'

export interface GameRoomMockData {
  gameId: number
  snapshot: GameSnapshot
  playerNames: [string, string, string, string]
  viewerSeat: Seat
  viewerHand: Card[]
  lastSyncedAt: string
  hostSeat: Seat
}

const seating: [SeatPublic, SeatPublic, SeatPublic, SeatPublic] = [
  {
    seat: 0,
    user_id: 101,
    display_name: 'You',
    is_ai: false,
    is_ready: true,
  },
  {
    seat: 1,
    user_id: 202,
    display_name: 'Bailey Bot',
    is_ai: true,
    is_ready: true,
  },
  {
    seat: 2,
    user_id: 303,
    display_name: 'Casey Bot',
    is_ai: true,
    is_ready: true,
  },
  {
    seat: 3,
    user_id: 404,
    display_name: 'Dakota Bot',
    is_ai: true,
    is_ready: true,
  },
]

const mockSnapshot: GameSnapshot = {
  game: {
    round_no: 5,
    dealer: 3,
    seating,
    scores_total: [42, 35, 27, 48],
    host_seat: 0,
  },
  phase: {
    phase: 'Trick',
    data: {
      round: {
        hand_size: 8,
        leader: 1,
        bid_winner: 3,
        trump: 'HEARTS',
        tricks_won: [2, 1, 0, 2],
      },
      trick_no: 3,
      leader: 1,
      current_trick: [
        [1, 'JD'],
        [2, 'KH'],
        [3, '7H'],
      ],
      to_act: 0,
      playable: ['2H', '5H', '8H', 'AS'],
    },
  },
}

const viewerHand: Card[] = ['2H', '5H', '8H', 'AS', 'TD']

const playerNames: [string, string, string, string] = [
  'You',
  'Bailey Bot',
  'Casey Bot',
  'Dakota Bot',
]

export function getMockGameRoomData(gameId: number): GameRoomMockData {
  return {
    gameId,
    snapshot: mockSnapshot,
    playerNames,
    viewerSeat: 0,
    viewerHand,
    lastSyncedAt: new Date('2025-01-06T15:04:05Z').toISOString(),
    hostSeat: 0,
  }
}
