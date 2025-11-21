import type {
  Card,
  PhaseSnapshot,
  RoundPublic,
  Seat,
} from '@/lib/game-room/types'

export interface SeatSummary {
  seat: Seat
  orientation: 'top' | 'left' | 'right' | 'bottom'
  name: string
  score: number
  isViewer: boolean
  isActive: boolean
  tricksWon?: number
  currentCard?: Card
  bid?: number | null
}

export function getOrientation(
  viewerSeat: Seat,
  seat: Seat
): SeatSummary['orientation'] {
  const relative = (seat - viewerSeat + 4) % 4
  if (relative === 0) return 'bottom'
  if (relative === 1) return 'left'
  if (relative === 2) return 'top'
  return 'right'
}

export function getPhaseLabel(phase: PhaseSnapshot): string {
  switch (phase.phase) {
    case 'Init':
      return 'Initializing'
    case 'Bidding':
      return 'Bidding Round'
    case 'TrumpSelect':
      return 'Select Trump'
    case 'Trick':
      return 'Trick Play'
    case 'Scoring':
      return 'Round Scoring'
    case 'Complete':
      return 'Round Complete'
    case 'GameOver':
      return 'Game Over'
    default:
      return 'Unknown Phase'
  }
}

export function getRound(phase: PhaseSnapshot): RoundPublic | null {
  switch (phase.phase) {
    case 'Bidding':
    case 'TrumpSelect':
    case 'Trick':
    case 'Scoring':
    case 'Complete':
      return phase.data.round
    default:
      return null
  }
}

export function getActiveSeat(phase: PhaseSnapshot): Seat | null {
  switch (phase.phase) {
    case 'Bidding':
    case 'TrumpSelect':
    case 'Trick':
      return phase.data.to_act
    default:
      return null
  }
}

export function getCurrentTrickMap(phase: PhaseSnapshot): Map<Seat, Card> {
  if (phase.phase !== 'Trick') {
    return new Map()
  }
  return new Map(phase.data.current_trick)
}

export function getBidForSeat(
  phase: PhaseSnapshot,
  seat: Seat,
  round?: RoundPublic | null
): number | null | undefined {
  if (phase.phase === 'Bidding') {
    return phase.data.bids[seat]
  }
  if (round?.bids) {
    return round.bids[seat]
  }
  return undefined
}

export function formatTrump(trump: RoundPublic['trump']): string {
  if (!trump) {
    return 'Undeclared'
  }

  switch (trump) {
    case 'CLUBS':
      return 'Clubs'
    case 'DIAMONDS':
      return 'Diamonds'
    case 'HEARTS':
      return 'Hearts'
    case 'SPADES':
      return 'Spades'
    case 'NO_TRUMP':
      return 'No Trump'
    default:
      return trump
  }
}

/**
 * Shortens a name to fit within a maximum length using progressive strategies.
 * Tries abbreviation strategies first, then truncates as a last resort.
 *
 * @param name - The full name to shorten
 * @param maxLength - Maximum character length allowed
 * @returns Shortened name that fits within maxLength
 */
export function shortenNameForDisplay(name: string, maxLength: number): string {
  // Step 1: Full name (if it fits)
  if (name.length <= maxLength) {
    return name
  }

  const words = name.trim().split(/\s+/)
  const wordCount = words.length

  // Step 2: Abbreviate middle/last name
  if (wordCount >= 3) {
    // "First Middle Last" -> "First M. Last"
    const abbreviated = `${words[0]} ${words[1][0]}. ${words[wordCount - 1]}`
    if (abbreviated.length <= maxLength) {
      return abbreviated
    }
  } else if (wordCount === 2) {
    // "First Last" -> "First L."
    const abbreviated = `${words[0]} ${words[1][0]}.`
    if (abbreviated.length <= maxLength) {
      return abbreviated
    }
  }

  // Step 3: First initial + last name
  if (wordCount >= 2) {
    // "First Last" or "First Middle Last" -> "F. Last"
    const firstInitial = `${words[0][0]}. ${words[wordCount - 1]}`
    if (firstInitial.length <= maxLength) {
      return firstInitial
    }
  }

  // Step 4: First name only
  if (wordCount >= 2) {
    const firstName = words[0]
    if (firstName.length <= maxLength) {
      return firstName
    }
  }

  // Step 5: Initials
  if (wordCount >= 2) {
    // "First Last" -> "FL"
    const initials = words.map((w) => w[0]).join('')
    if (initials.length <= maxLength) {
      return initials
    }
  }

  // Step 6: Final fallback - truncate to maxLength or 8, whichever is smaller (no ellipsis)
  const truncateLength = Math.min(maxLength, 8)
  const truncated = name.substring(0, truncateLength)
  return truncated
}

export function buildSeatSummaries(params: {
  playerNames: [string, string, string, string]
  viewerSeat: Seat
  phase: PhaseSnapshot
  scores: [number, number, number, number]
  trickMap: Map<Seat, Card>
  round: RoundPublic | null
  activeSeat: Seat | null
  actualViewerSeat?: Seat | null
}): SeatSummary[] {
  const {
    playerNames,
    viewerSeat,
    phase,
    scores,
    trickMap,
    round,
    activeSeat,
    actualViewerSeat,
  } = params

  // Use actualViewerSeat if provided (for isViewer check), otherwise use viewerSeat (for orientation)
  const viewerSeatForCheck =
    actualViewerSeat !== undefined ? actualViewerSeat : viewerSeat

  return [0, 1, 2, 3].map((seat) => {
    const orientation = getOrientation(viewerSeat, seat as Seat)
    const isViewer = viewerSeatForCheck !== null && seat === viewerSeatForCheck
    const tricksWon = round?.tricks_won[seat as Seat]
    const currentCard = trickMap.get(seat as Seat)
    const bid = getBidForSeat(phase, seat as Seat, round)
    const isActive = activeSeat === seat

    return {
      seat: seat as Seat,
      orientation,
      name: playerNames[seat as Seat],
      score: scores[seat as Seat],
      isViewer,
      tricksWon,
      currentCard,
      bid,
      isActive,
    }
  })
}
