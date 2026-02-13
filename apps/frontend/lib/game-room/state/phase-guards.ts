import type {
  BiddingSnapshot,
  PhaseSnapshot,
  TrumpSelectSnapshot,
  TrickSnapshot,
} from '@/lib/game-room/types'

/**
 * Phase type guards for lib consumers (mutations, hooks).
 * Live in lib so hooks need not depend on app route components.
 */
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
