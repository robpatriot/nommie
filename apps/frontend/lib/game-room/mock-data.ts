import type { Card, GameSnapshot, Seat } from './types'

export interface GameRoomMockData {
  gameId: number
  snapshot: GameSnapshot
  playerNames: [string, string, string, string]
  viewerSeat: Seat
  viewerHand: Card[]
  lastSyncedAt: string
}

const mockSnapshot: GameSnapshot = {
  game: {
    round_no: 5,
    dealer: 3,
    seating: [101, 202, 303, 404],
    scores_total: [42, 35, 27, 48],
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
  'Bailey',
  'Casey',
  'Dakota',
]

export function getMockGameRoomData(gameId: number): GameRoomMockData {
  return {
    gameId,
    snapshot: mockSnapshot,
    playerNames,
    viewerSeat: 0,
    viewerHand,
    lastSyncedAt: new Date('2025-01-06T15:04:05Z').toISOString(),
  }
}
