export type Seat = 0 | 1 | 2 | 3

export type Trump = 'CLUBS' | 'DIAMONDS' | 'HEARTS' | 'SPADES' | 'NO_TRUMP'

export type Card = string

export interface SeatPublic {
  seat: Seat
  user_id: number | null
  display_name: string | null
  is_ai: boolean
  is_ready: boolean
  ai_profile?: {
    name: string
    version: string
  } | null
}

export interface GameHeader {
  round_no: number
  dealer: Seat
  seating: [SeatPublic, SeatPublic, SeatPublic, SeatPublic]
  scores_total: [number, number, number, number]
  host_seat: Seat
}

export interface RoundPublic {
  hand_size: number
  leader: Seat
  bid_winner: Seat | null
  trump: Trump | null
  tricks_won: [number, number, number, number]
}

export interface BiddingSnapshot {
  round: RoundPublic
  to_act: Seat
  bids: [number | null, number | null, number | null, number | null]
  min_bid: number
  max_bid: number
}

export interface TrumpSelectSnapshot {
  round: RoundPublic
  to_act: Seat
  allowed_trumps: Trump[]
}

export interface TrickSnapshot {
  round: RoundPublic
  trick_no: number
  leader: Seat
  current_trick: Array<[Seat, Card]>
  to_act: Seat
  playable: Card[]
}

export interface ScoringSnapshot {
  round: RoundPublic
  round_scores: [number, number, number, number]
}

export interface CompleteSnapshot {
  round: RoundPublic
}

export type PhaseSnapshot =
  | { phase: 'Init' }
  | { phase: 'Bidding'; data: BiddingSnapshot }
  | { phase: 'TrumpSelect'; data: TrumpSelectSnapshot }
  | { phase: 'Trick'; data: TrickSnapshot }
  | { phase: 'Scoring'; data: ScoringSnapshot }
  | { phase: 'Complete'; data: CompleteSnapshot }
  | { phase: 'GameOver' }

export interface GameSnapshot {
  game: GameHeader
  phase: PhaseSnapshot
}
