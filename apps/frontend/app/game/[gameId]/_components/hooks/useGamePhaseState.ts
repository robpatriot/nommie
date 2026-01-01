import { useMemo } from 'react'
import type { PhaseSnapshot } from '@/lib/game-room/types'
import {
  isInitPhase,
  isGameOverPhase,
  isActiveGame,
  getPhaseTranslationKey,
} from '../game-room/phase-helpers'

/**
 * Hook to consolidate phase state checks and derived values.
 * Provides memoized phase state information for use in components.
 */
export function useGamePhaseState(phase: PhaseSnapshot) {
  return useMemo(
    () => ({
      isPreGame: isInitPhase(phase),
      isGameOver: isGameOverPhase(phase),
      isActive: isActiveGame(phase),
      phaseName: phase.phase,
      translationKey: getPhaseTranslationKey(phase.phase),
    }),
    [phase]
  )
}
