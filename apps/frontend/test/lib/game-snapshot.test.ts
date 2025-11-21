import { describe, expect, expectTypeOf, it } from 'vitest'

import type { GameSnapshot, PhaseSnapshot } from '@/lib/game-room/types'
import {
  biddingSnapshotFixture,
  gameOverPhaseSnapshot,
  initPhaseSnapshot,
  scoringSnapshotFixture,
  trickSnapshotFixture,
} from '../mocks/game-snapshot'

describe('GameSnapshot contract', () => {
  it('serializes and parses bidding snapshot data', () => {
    const json = JSON.stringify(biddingSnapshotFixture)
    const parsed = JSON.parse(json) as GameSnapshot

    expectTypeOf(parsed).toMatchTypeOf<GameSnapshot>()
    expect(parsed.phase.phase).toBe('Bidding')
    if (parsed.phase.phase === 'Bidding') {
      expect(parsed.phase.data.bids).toHaveLength(4)
      expect(parsed.phase.data.round.bids).toHaveLength(4)
      expect(parsed.phase.data.previous_round?.bids).toEqual([2, 3, 1, 1])
      expect(parsed.phase.data.previous_round?.tricks_won).toEqual([2, 1, 0, 1])
      expect(parsed.phase.data.min_bid).toBeLessThanOrEqual(
        parsed.phase.data.max_bid
      )
    }
  })

  it('supports trick snapshot tuple payloads', () => {
    const json = JSON.stringify(trickSnapshotFixture)
    const parsed = JSON.parse(json) as GameSnapshot

    expect(parsed.phase.phase).toBe('Trick')
    if (parsed.phase.phase === 'Trick') {
      expect(parsed.phase.data.current_trick[0]).toEqual([2, 'AS'])
      expect(parsed.phase.data.playable).toContain('KD')
      expect(parsed.phase.data.round.bids).toEqual([2, 4, 5, 3])
    }
  })

  it('covers terminal phases as discriminated unions', () => {
    const phases: PhaseSnapshot[] = [
      initPhaseSnapshot,
      biddingSnapshotFixture.phase,
      trickSnapshotFixture.phase,
      scoringSnapshotFixture.phase,
      gameOverPhaseSnapshot,
    ]

    expect(phases.map((p) => p.phase)).toEqual([
      'Init',
      'Bidding',
      'Trick',
      'Scoring',
      'GameOver',
    ])
  })

  it('ensures scoring payload retains round metadata', () => {
    const { phase } = scoringSnapshotFixture

    if (phase.phase !== 'Scoring') {
      throw new Error('Fixture must be scoring variant')
    }

    expect(phase.data.round.trump).toBe('NO_TRUMP')
    expect(
      phase.data.round.tricks_won.reduce((sum, value) => sum + value, 0)
    ).toBeGreaterThan(0)
    expect(phase.data.round_scores).toHaveLength(4)
  })
})
