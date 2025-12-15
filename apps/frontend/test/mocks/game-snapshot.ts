import type {
  BiddingSnapshot,
  GameSnapshot,
  PhaseSnapshot,
  RoundPublic,
  SeatPublic,
  TrickSnapshot,
} from '@/lib/game-room/types'

const baseRound: RoundPublic = {
  hand_size: 8,
  leader: 0,
  bid_winner: null,
  trump: null,
  tricks_won: [0, 0, 0, 0],
  bids: [null, null, null, null],
}

const seatingFixture: [SeatPublic, SeatPublic, SeatPublic, SeatPublic] = [
  {
    seat: 0,
    user_id: 101,
    display_name: 'Alex',
    is_ai: false,
    is_ready: false,
  },
  {
    seat: 1,
    user_id: 202,
    display_name: 'Bailey Bot',
    is_ai: true,
    is_ready: false,
  },
  {
    seat: 2,
    user_id: 303,
    display_name: 'Casey Bot',
    is_ai: true,
    is_ready: false,
  },
  {
    seat: 3,
    user_id: 404,
    display_name: 'Dakota Bot',
    is_ai: true,
    is_ready: false,
  },
]

export const biddingSnapshotFixture: GameSnapshot = {
  game: {
    round_no: 1,
    dealer: 0,
    seating: seatingFixture,
    scores_total: [0, 0, 0, 0],
    host_seat: 0,
  },
  phase: {
    phase: 'Bidding',
    data: {
      round: baseRound,
      to_act: 1,
      bids: [2, null, null, null],
      min_bid: 0,
      max_bid: 8,
      last_trick: [
        [0, '2H'],
        [1, '3C'],
        [2, '4D'],
        [3, '5S'],
      ],
      previous_round: {
        round_no: 0,
        hand_size: 8,
        tricks_won: [2, 1, 0, 1],
        bids: [2, 3, 1, 1],
      },
    },
  },
}

export const initSnapshotFixture: GameSnapshot = {
  game: {
    round_no: 0,
    dealer: 0,
    seating: seatingFixture,
    scores_total: [0, 0, 0, 0],
    host_seat: 0,
  },
  phase: {
    phase: 'Init',
  },
}

export const trickSnapshotFixture: GameSnapshot = {
  game: {
    round_no: 5,
    dealer: 0,
    seating: seatingFixture,
    scores_total: [12, 8, 14, 16],
    host_seat: 0,
  },
  phase: {
    phase: 'Trick',
    data: {
      round: {
        ...baseRound,
        bid_winner: 3,
        trump: 'HEARTS',
        tricks_won: [1, 0, 2, 1],
        bids: [2, 4, 5, 3],
      },
      trick_no: 3,
      leader: 2,
      current_trick: [
        [2, 'AS'],
        [3, 'TH'],
      ],
      to_act: 0,
      playable: ['2H', 'KD', 'QC'],
      last_trick: null,
    },
  },
}

export const scoringSnapshotFixture: GameSnapshot = {
  game: {
    round_no: 8,
    dealer: 3,
    seating: seatingFixture,
    scores_total: [42, 35, 27, 48],
    host_seat: 0,
  },
  phase: {
    phase: 'Scoring',
    data: {
      round: {
        ...baseRound,
        hand_size: 5,
        bid_winner: 0,
        trump: 'NO_TRUMPS',
        tricks_won: [3, 1, 0, 1],
      },
      round_scores: [12, -3, -5, 4],
    },
  },
}

export const initPhaseSnapshot: PhaseSnapshot = { phase: 'Init' }

export const gameOverPhaseSnapshot: PhaseSnapshot = { phase: 'GameOver' }

export const biddingPhaseSnapshot: BiddingSnapshot =
  biddingSnapshotFixture.phase.phase === 'Bidding'
    ? biddingSnapshotFixture.phase.data
    : {
        round: baseRound,
        to_act: 1,
        bids: [2, null, null, null],
        min_bid: 0,
        max_bid: 8,
        last_trick: [
          [0, '2H'],
          [1, '3C'],
          [2, '4D'],
          [3, '5S'],
        ],
        previous_round: {
          round_no: 0,
          hand_size: 8,
          tricks_won: [2, 1, 0, 1],
          bids: [2, 3, 1, 1],
        },
      }

export const trickPhaseSnapshot: TrickSnapshot =
  trickSnapshotFixture.phase.phase === 'Trick'
    ? trickSnapshotFixture.phase.data
    : {
        round: baseRound,
        trick_no: 1,
        leader: 0,
        current_trick: [],
        to_act: 0,
        playable: [],
        last_trick: null,
      }
