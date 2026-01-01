import type {
  BiddingSnapshot,
  RoundPublic,
  TrickSnapshot,
  TrumpSelectSnapshot,
} from '@/lib/game-room/types'

// Shared base round data
const baseRound: RoundPublic = {
  hand_size: 8,
  leader: 0,
  bid_winner: null,
  trump: null,
  tricks_won: [0, 0, 0, 0],
  bids: [null, null, null, null],
}

const nullBids = [null, null, null, null] as [
  number | null,
  number | null,
  number | null,
  number | null,
]

/**
 * Factory for creating BiddingSnapshot test data
 */
export function createBiddingPhase(
  overrides?: Partial<BiddingSnapshot>
): BiddingSnapshot {
  return {
    round: {
      ...baseRound,
    },
    to_act: 0,
    bids: nullBids,
    min_bid: 0,
    max_bid: 8,
    last_trick: null,
    previous_round: null,
    ...overrides,
  }
}

/**
 * Factory for creating TrumpSelectSnapshot test data
 */
export function createTrumpPhase(
  overrides?: Partial<TrumpSelectSnapshot>
): TrumpSelectSnapshot {
  return {
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
    allowed_trumps: ['HEARTS', 'SPADES', 'NO_TRUMPS'],
    last_trick: null,
    ...overrides,
  }
}

/**
 * Factory for creating TrickSnapshot test data
 */
export function createTrickPhase(
  overrides?: Partial<TrickSnapshot>
): TrickSnapshot {
  return {
    round: {
      hand_size: 8,
      leader: 0,
      bid_winner: 0,
      trump: 'HEARTS',
      tricks_won: [0, 0, 0, 0],
      bids: [2, 3, 4, 5] as [
        number | null,
        number | null,
        number | null,
        number | null,
      ],
    },
    trick_no: 1,
    leader: 0,
    current_trick: [],
    to_act: 0,
    playable: ['2H', '3C', '5S'],
    last_trick: null,
    ...overrides,
  }
}
