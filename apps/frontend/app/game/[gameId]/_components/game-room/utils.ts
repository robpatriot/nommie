import type {
  Card,
  PhaseSnapshot,
  RoundPublic,
  Seat,
} from '@/lib/game-room/types'
import { CARD_DIMENSIONS } from './PlayingCard'

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

/**
 * Order for sorting seat summaries by orientation (for mobile display).
 * Bottom (viewer) first, then right, top, left.
 */
export const ORIENTATION_ORDER_MOBILE: SeatSummary['orientation'][] = [
  'bottom',
  'right',
  'top',
  'left',
]

/**
 * Order for sorting cards in trick area by orientation.
 * Left, top, right, bottom (play order around the table).
 */
export const ORIENTATION_ORDER_TRICK: Array<
  'bottom' | 'right' | 'top' | 'left'
> = ['left', 'top', 'right', 'bottom']

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
    // "First Middle Last" -> "First M Last"
    const abbreviated = `${words[0]} ${words[1][0]} ${words[wordCount - 1]}`
    if (abbreviated.length <= maxLength) {
      return abbreviated
    }
  } else if (wordCount === 2) {
    // "First Last" -> "First L"
    const abbreviated = `${words[0]} ${words[1][0]}`
    if (abbreviated.length <= maxLength) {
      return abbreviated
    }
  }

  // Step 3: First initial + last name
  if (wordCount >= 2) {
    // "First Last" or "First Middle Last" -> "F Last"
    const firstInitial = `${words[0][0]} ${words[wordCount - 1]}`
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
  historicalStats?: {
    bids?: [number | null, number | null, number | null, number | null]
    tricksWon?: [number, number, number, number]
  }
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
    historicalStats,
  } = params

  // Use actualViewerSeat if provided (for isViewer check), otherwise use viewerSeat (for orientation)
  const viewerSeatForCheck =
    actualViewerSeat !== undefined ? actualViewerSeat : viewerSeat

  return [0, 1, 2, 3].map((seat) => {
    const orientation = getOrientation(viewerSeat, seat as Seat)
    const isViewer = viewerSeatForCheck !== null && seat === viewerSeatForCheck
    const historicalTricks = historicalStats?.tricksWon?.[seat as Seat]
    const tricksWon =
      historicalTricks !== undefined
        ? historicalTricks
        : round?.tricks_won[seat as Seat]
    const currentCard = trickMap.get(seat as Seat)
    const historicalBid = historicalStats?.bids?.[seat as Seat]
    const bid =
      historicalBid !== undefined
        ? historicalBid
        : getBidForSeat(phase, seat as Seat, round)
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

// Scaling constants - matching PlayerHand scaling logic
const CARD_WIDTH = CARD_DIMENSIONS.md.width
const CARD_PADDING = 8
const EFFECTIVE_CARD_WIDTH = CARD_WIDTH + CARD_PADDING
const MIN_OVERLAP = 4
const MAX_OVERLAP = CARD_WIDTH - 17
const AESTHETIC_OVERLAP = 8
const BLEND_THRESHOLD = 20
const MIN_SCALE = 0.75

// Calculate overlap for a single row given viewport width and card count
function calculateOverlapForRow(
  viewportWidth: number,
  cardCount: number
): number {
  if (cardCount <= 1) {
    return 0
  }

  const totalWidthNeeded = EFFECTIVE_CARD_WIDTH * cardCount
  const spaceRemaining = viewportWidth - totalWidthNeeded

  if (spaceRemaining >= BLEND_THRESHOLD) {
    return AESTHETIC_OVERLAP
  }

  if (spaceRemaining > 0) {
    const overlapNeeded = (totalWidthNeeded - viewportWidth) / (cardCount - 1)
    const calculatedOverlap = Math.max(
      MIN_OVERLAP,
      Math.min(MAX_OVERLAP, overlapNeeded)
    )
    const blend = 1 - spaceRemaining / BLEND_THRESHOLD
    return AESTHETIC_OVERLAP + (calculatedOverlap - AESTHETIC_OVERLAP) * blend
  }

  const overlapNeeded = (totalWidthNeeded - viewportWidth) / (cardCount - 1)
  return Math.max(MIN_OVERLAP, Math.min(MAX_OVERLAP, overlapNeeded))
}

// Calculate the minimum width needed for a row to reach max overlap
function calculateMinWidthForMaxOverlap(cardCount: number): number {
  if (cardCount <= 1) {
    return EFFECTIVE_CARD_WIDTH
  }
  return (
    EFFECTIVE_CARD_WIDTH +
    (cardCount - 1) * (EFFECTIVE_CARD_WIDTH - MAX_OVERLAP)
  )
}

// Check if cards fit in one row with acceptable overlap
function canFitInOneRow(viewportWidth: number, cardCount: number): boolean {
  if (cardCount <= 1) {
    return true
  }
  return viewportWidth >= calculateMinWidthForMaxOverlap(cardCount)
}

/**
 * Calculate card scale factor based on viewport width and card count.
 * This matches the logic used in PlayerHand's ScaledLayoutStrategy.
 * Returns a scale factor between MIN_SCALE (0.75) and 1.0.
 */
export function calculateCardScale(
  viewportWidth: number,
  cardCount: number
): number {
  if (cardCount === 0) {
    return 1
  }

  // Check if we can fit in one row
  if (canFitInOneRow(viewportWidth, cardCount)) {
    return 1 // Single row mode: no scaling
  }

  // Two-row mode: check if scaling is needed
  const topRowCount = Math.ceil(cardCount / 2)
  const bottomRowCount = cardCount - topRowCount

  // Check overlaps with actual viewport to determine if max overlap is reached
  const topOverlap = calculateOverlapForRow(viewportWidth, topRowCount)
  const bottomOverlap =
    bottomRowCount > 0
      ? calculateOverlapForRow(viewportWidth, bottomRowCount)
      : 0

  const topRowAtMaxOverlap =
    topRowCount > 1 && Math.abs(topOverlap - MAX_OVERLAP) < 0.1
  const bottomRowAtMaxOverlap =
    bottomRowCount > 1 && Math.abs(bottomOverlap - MAX_OVERLAP) < 0.1
  const eitherRowAtMaxOverlap = topRowAtMaxOverlap || bottomRowAtMaxOverlap

  // Calculate the width where max overlap would be reached
  const minWidthForTwoRowMaxOverlap = Math.max(
    calculateMinWidthForMaxOverlap(topRowCount),
    calculateMinWidthForMaxOverlap(bottomRowCount)
  )

  // Determine scale: only scale when max overlap is reached and viewport is below threshold
  if (eitherRowAtMaxOverlap && viewportWidth < minWidthForTwoRowMaxOverlap) {
    return Math.max(MIN_SCALE, viewportWidth / minWidthForTwoRowMaxOverlap)
  }

  return 1
}
