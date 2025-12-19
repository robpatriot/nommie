import type {
  GameHistorySummary,
  RoundHistoryEntry,
  Seat,
  Trump,
} from '@/lib/game-room/types'
import { isValidSeat } from '@/utils/seat-validation'
import type {
  GameHistoryApiResponse,
  GameHistoryApiRound,
} from '@/app/actions/game-actions'

/**
 * Map API response to GameHistorySummary format.
 */
export function mapGameHistory(
  response: GameHistoryApiResponse
): GameHistorySummary {
  const rounds = response.rounds.map(mapRound)
  return { rounds }
}

function mapRound(round: GameHistoryApiRound): RoundHistoryEntry {
  return {
    roundNo: round.round_no,
    handSize: round.hand_size,
    dealerSeat: ensureSeat(round.dealer_seat),
    trumpSelectorSeat: toSeatOrNull(round.trump_selector_seat),
    trump: round.trump as Trump | null,
    bids: round.bids,
    cumulativeScores: round.cumulative_scores,
  }
}

function ensureSeat(value: number): Seat {
  if (isValidSeat(value)) {
    return value as Seat
  }
  console.warn(`Invalid seat index from history payload: ${value}`)
  return 0
}

function toSeatOrNull(value: number | null | undefined): Seat | null {
  if (typeof value !== 'number') {
    return null
  }
  return isValidSeat(value) ? (value as Seat) : null
}
