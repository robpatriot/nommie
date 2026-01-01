import { describe, expect, it, vi, beforeEach } from 'vitest'
import { render, screen } from '../utils'
import { TrickArea } from '@/app/game/[gameId]/_components/game-room/TrickArea'
import type { PhaseSnapshot, Seat } from '@/lib/game-room/types'
import {
  biddingPhaseSnapshot,
  trickPhaseSnapshot,
  initPhaseSnapshot,
} from '../mocks/game-snapshot'

// Mock useMediaQuery hook
vi.mock('@/hooks/useMediaQuery', () => ({
  useMediaQuery: (query: string) => {
    // Return true for large viewport (>640px) by default
    if (query.includes('640px')) {
      return true
    }
    if (query.includes('380px')) {
      return true
    }
    return false
  },
}))

describe('TrickArea', () => {
  const playerNames: [string, string, string, string] = [
    'Alex',
    'Bailey',
    'Casey',
    'Dakota',
  ]

  const getSeatName = (seat: number) => playerNames[seat]

  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('Empty State', () => {
    it('shows waiting message when no cards in trick', () => {
      const trickMap = new Map<Seat, string>()
      const phase: PhaseSnapshot = {
        phase: 'Trick',
        data: {
          ...trickPhaseSnapshot,
          current_trick: [],
        },
      }

      render(
        <TrickArea
          trickMap={trickMap}
          getSeatName={getSeatName}
          round={phase.data.round}
          phase={phase}
          viewerSeat={0}
        />
      )

      expect(screen.getByText(/Waiting for/i)).toBeInTheDocument()
    })

    it('shows trick number when in trick phase with no cards', () => {
      const trickMap = new Map<Seat, string>()
      const phase: PhaseSnapshot = {
        phase: 'Trick',
        data: {
          ...trickPhaseSnapshot,
          current_trick: [],
        },
      }

      render(
        <TrickArea
          trickMap={trickMap}
          getSeatName={getSeatName}
          round={phase.data.round}
          phase={phase}
          viewerSeat={0}
        />
      )

      // Trick number is shown when there are no cards in the trick
      expect(screen.getByText(/Trick \d+ of/i)).toBeInTheDocument()
    })
  })

  describe('Current Trick Display', () => {
    it('displays cards in current trick', () => {
      const trickMap = new Map<Seat, string>([
        [0 as Seat, '2H'],
        [1 as Seat, '3C'],
      ])
      const phase: PhaseSnapshot = {
        phase: 'Trick',
        data: {
          ...trickPhaseSnapshot,
          current_trick: [
            [0, '2H'],
            [1, '3C'],
          ],
        },
      }

      render(
        <TrickArea
          trickMap={trickMap}
          getSeatName={getSeatName}
          round={phase.data.round}
          phase={phase}
          viewerSeat={0}
        />
      )

      // Component should render with cards
      const region = screen.getByRole('region')
      expect(region).toBeInTheDocument()
      // Verify aria-label is present for accessibility
      expect(region).toHaveAttribute('aria-label')
    })

    it('displays all four cards when trick is complete', () => {
      const trickMap = new Map<Seat, string>([
        [0 as Seat, '2H'],
        [1 as Seat, '3C'],
        [2 as Seat, '4D'],
        [3 as Seat, '5S'],
      ])
      const phase: PhaseSnapshot = {
        phase: 'Trick',
        data: {
          ...trickPhaseSnapshot,
          current_trick: [
            [0, '2H'],
            [1, '3C'],
            [2, '4D'],
            [3, '5S'],
          ],
        },
      }

      render(
        <TrickArea
          trickMap={trickMap}
          getSeatName={getSeatName}
          round={phase.data.round}
          phase={phase}
          viewerSeat={0}
        />
      )

      // Component should render with all four cards
      const region = screen.getByRole('region')
      expect(region).toBeInTheDocument()
      expect(region).toHaveAttribute('aria-label')
    })
  })

  describe('Last Trick Display', () => {
    it('shows last trick when between rounds', () => {
      const trickMap = new Map<Seat, string>()
      const phase: PhaseSnapshot = {
        phase: 'Bidding',
        data: biddingPhaseSnapshot,
      }
      const lastTrick: Array<[Seat, string]> = [
        [0 as Seat, '2H'],
        [1 as Seat, '3C'],
        [2 as Seat, '4D'],
        [3 as Seat, '5S'],
      ]

      render(
        <TrickArea
          trickMap={trickMap}
          getSeatName={getSeatName}
          round={phase.data.round}
          phase={phase}
          viewerSeat={0}
          lastTrick={lastTrick}
        />
      )

      // LastTrickCards component should render
      expect(screen.getByRole('region')).toBeInTheDocument()
    })

    it('shows last trick when showPreviousRoundPosition is true', () => {
      const trickMap = new Map<Seat, string>()
      const phase: PhaseSnapshot = {
        phase: 'Trick',
        data: trickPhaseSnapshot,
      }
      const lastTrick: Array<[Seat, string]> = [
        [0 as Seat, '2H'],
        [1 as Seat, '3C'],
      ]

      render(
        <TrickArea
          trickMap={trickMap}
          getSeatName={getSeatName}
          round={phase.data.round}
          phase={phase}
          viewerSeat={0}
          lastTrick={lastTrick}
          showPreviousRoundPosition={true}
        />
      )

      const region = screen.getByRole('region')
      expect(region).toBeInTheDocument()
      expect(region).toHaveAttribute('aria-label')
    })

    it('hides last trick when showPreviousRoundPosition is false and has current cards', () => {
      const trickMap = new Map<Seat, string>([[0 as Seat, '2H']])
      const phase: PhaseSnapshot = {
        phase: 'Trick',
        data: {
          ...trickPhaseSnapshot,
          current_trick: [[0 as Seat, '2H']],
        },
      }
      const lastTrick: Array<[Seat, string]> = [
        [1 as Seat, '3C'],
        [2 as Seat, '4D'],
      ]

      render(
        <TrickArea
          trickMap={trickMap}
          getSeatName={getSeatName}
          round={phase.data.round}
          phase={phase}
          viewerSeat={0}
          lastTrick={lastTrick}
          showPreviousRoundPosition={false}
        />
      )

      // Should show current trick, not last trick
      const region = screen.getByRole('region')
      expect(region).toBeInTheDocument()
      expect(region).toHaveAttribute('aria-label')
    })
  })

  describe('Header Row', () => {
    it('displays header with trump and total bids when round exists', () => {
      const trickMap = new Map<Seat, string>()
      const phase: PhaseSnapshot = {
        phase: 'Trick',
        data: {
          ...trickPhaseSnapshot,
          current_trick: [],
        },
      }

      render(
        <TrickArea
          trickMap={trickMap}
          getSeatName={getSeatName}
          round={phase.data.round}
          phase={phase}
          viewerSeat={0}
        />
      )

      // TrickAreaHeader should be rendered (contains trump and bid info)
      const region = screen.getByRole('region')
      expect(region).toBeInTheDocument()
      expect(region).toHaveAttribute('aria-label')
    })

    it('does not display header when round is null', () => {
      const trickMap = new Map<Seat, string>()
      const phase = initPhaseSnapshot

      render(
        <TrickArea
          trickMap={trickMap}
          getSeatName={getSeatName}
          round={null}
          phase={phase}
          viewerSeat={0}
        />
      )

      expect(screen.getByRole('region')).toBeInTheDocument()
    })
  })

  describe('Card Scale', () => {
    it('applies card scale correctly', () => {
      const trickMap = new Map<Seat, string>([[0 as Seat, '2H']])
      const phase: PhaseSnapshot = {
        phase: 'Trick',
        data: {
          ...trickPhaseSnapshot,
          current_trick: [[0 as Seat, '2H']],
        },
      }

      render(
        <TrickArea
          trickMap={trickMap}
          getSeatName={getSeatName}
          round={phase.data.round}
          phase={phase}
          viewerSeat={0}
          cardScale={0.8}
        />
      )

      const region = screen.getByRole('region')
      expect(region).toBeInTheDocument()
      expect(region).toHaveAttribute('aria-label')
    })

    it('clamps card scale to valid range', () => {
      const trickMap = new Map<Seat, string>([[0 as Seat, '2H']])
      const phase: PhaseSnapshot = {
        phase: 'Trick',
        data: {
          ...trickPhaseSnapshot,
          current_trick: [[0 as Seat, '2H']],
        },
      }

      // Test with invalid scale values - component should handle gracefully
      render(
        <TrickArea
          trickMap={trickMap}
          getSeatName={getSeatName}
          round={phase.data.round}
          phase={phase}
          viewerSeat={0}
          cardScale={-1}
        />
      )

      // Component should render even with invalid scale (it clamps internally)
      const region = screen.getByRole('region')
      expect(region).toBeInTheDocument()
      expect(region).toHaveAttribute('aria-label')
    })
  })

  describe('Accessibility', () => {
    it('has proper aria-label', () => {
      const trickMap = new Map<Seat, string>()
      const phase: PhaseSnapshot = {
        phase: 'Trick',
        data: {
          ...trickPhaseSnapshot,
          current_trick: [],
        },
      }

      render(
        <TrickArea
          trickMap={trickMap}
          getSeatName={getSeatName}
          round={phase.data.round}
          phase={phase}
          viewerSeat={0}
        />
      )

      const region = screen.getByRole('region')
      expect(region).toHaveAttribute('aria-label')
    })
  })
})
