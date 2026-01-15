import type {
  BiddingSnapshot,
  Card,
  PhaseSnapshot,
  Seat,
  TrumpSelectSnapshot,
  TrickSnapshot,
} from '@/lib/game-room/types'

/**
 * Type-safe phase checking functions.
 * These use TypeScript type guards to narrow the phase type.
 */

export function isInitPhase(phase: PhaseSnapshot): phase is { phase: 'Init' } {
  return phase.phase === 'Init'
}

export function isGameOverPhase(
  phase: PhaseSnapshot
): phase is { phase: 'GameOver' } {
  return phase.phase === 'GameOver'
}

export function isBiddingPhase(
  phase: PhaseSnapshot
): phase is { phase: 'Bidding'; data: BiddingSnapshot } {
  return phase.phase === 'Bidding'
}

export function isTrumpSelectPhase(
  phase: PhaseSnapshot
): phase is { phase: 'TrumpSelect'; data: TrumpSelectSnapshot } {
  return phase.phase === 'TrumpSelect'
}

export function isTrickPhase(
  phase: PhaseSnapshot
): phase is { phase: 'Trick'; data: TrickSnapshot } {
  return phase.phase === 'Trick'
}

/**
 * Check if the game is active (not Init or GameOver)
 */
export function isActiveGame(phase: PhaseSnapshot): boolean {
  return !isInitPhase(phase) && !isGameOverPhase(phase)
}

/**
 * Get last trick from phase if available.
 * Returns null if phase doesn't have last trick data.
 */
export function getLastTrick(phase: PhaseSnapshot): Array<[Seat, Card]> | null {
  if (isBiddingPhase(phase)) {
    return phase.data.last_trick
  }
  if (isTrumpSelectPhase(phase)) {
    return phase.data.last_trick
  }
  if (isTrickPhase(phase)) {
    return phase.data.last_trick
  }
  return null
}

/**
 * Get historical stats from phase if available (previous round data).
 * Returns undefined if not available.
 */
export function getHistoricalStats(phase: PhaseSnapshot):
  | {
      bids: [number | null, number | null, number | null, number | null]
      tricksWon: [number, number, number, number]
    }
  | undefined {
  if (isBiddingPhase(phase) && phase.data.previous_round) {
    return {
      bids: phase.data.previous_round.bids,
      tricksWon: phase.data.previous_round.tricks_won,
    }
  }
  return undefined
}

/**
 * Get phase translation key for i18n.
 * Maps phase names to translation keys used in game.json.
 */
export function getPhaseTranslationKey(phase: PhaseSnapshot['phase']): string {
  switch (phase) {
    case 'Init':
      return 'init'
    case 'Bidding':
      return 'bidding'
    case 'TrumpSelect':
      return 'trumpSelect'
    case 'Trick':
      return 'trick'
    case 'Scoring':
      return 'scoring'
    case 'Complete':
      return 'complete'
    case 'GameOver':
      return 'gameOver'
    default:
      return 'unknown'
  }
}
