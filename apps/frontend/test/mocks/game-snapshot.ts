import type {
  BiddingSnapshot,
  GameSnapshot,
  PhaseSnapshot,
  RoundPublic,
  TrickSnapshot,
} from '@/lib/game-room/types'

const baseRound: RoundPublic = {
  hand_size: 8,
  leader: 0,
  bid_winner: null,
  trump: null,
  tricks_won: [0, 0, 0, 0],
}

export const biddingSnapshotFixture: GameSnapshot = {
  game: {
    round_no: 1,
    dealer: 0,
    seating: [101, 202, 303, 404],
    scores_total: [0, 0, 0, 0],
  },
  phase: {
    phase: 'Bidding',
    data: {
      round: baseRound,
      to_act: 1,
      bids: [2, null, null, null],
      min_bid: 0,
      max_bid: 8,
    },
  },
}

export const trickSnapshotFixture: GameSnapshot = {
  game: {
    round_no: 5,
    dealer: 0,
    seating: [101, 202, 303, 404],
    scores_total: [12, 8, 14, 16],
  },
  phase: {
    phase: 'Trick',
    data: {
      round: {
        ...baseRound,
        bid_winner: 3,
        trump: 'HEARTS',
        tricks_won: [1, 0, 2, 1],
      },
      trick_no: 3,
      leader: 2,
      current_trick: [
        [2, 'AS'],
        [3, 'TH'],
      ],
      to_act: 0,
      playable: ['2H', 'KD', 'QC'],
    },
  },
}

export const scoringSnapshotFixture: GameSnapshot = {
  game: {
    round_no: 8,
    dealer: 3,
    seating: [101, 202, 303, 404],
    scores_total: [42, 35, 27, 48],
  },
  phase: {
    phase: 'Scoring',
    data: {
      round: {
        ...baseRound,
        hand_size: 5,
        bid_winner: 0,
        trump: 'NO_TRUMP',
        tricks_won: [3, 1, 0, 1],
      },
      round_scores: [12, -3, -5, 4],
    },
  },
}

export const initPhaseSnapshot: PhaseSnapshot = { phase: 'Init' }

export const gameOverPhaseSnapshot: PhaseSnapshot = { phase: 'GameOver' }

export const biddingPhaseSnapshot: BiddingSnapshot =
  biddingSnapshotFixture.phase.data

export const trickPhaseSnapshot: TrickSnapshot = trickSnapshotFixture.phase.data
