import { beforeEach, describe, expect, it, vi } from 'vitest'
import userEvent from '@testing-library/user-event'

import { PlayerHand } from '@/app/game/[gameId]/_components/game-room/PlayerHand'
import type { PhaseSnapshot, Seat } from '@/lib/game-room/types'
import { render, screen } from '../utils'

// Mock useMediaQuery hook
vi.mock('@/hooks/useMediaQuery', () => ({
  useMediaQuery: (_query: string) => {
    // Return true for all media queries in tests (show all UI elements)
    return true
  },
}))

describe('PlayerHand', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })
  const playerNames: [string, string, string, string] = [
    'Alex',
    'Bailey',
    'Casey',
    'Dakota',
  ]

  const createTrickPhase = (
    toAct: Seat,
    currentTrick: Array<[Seat, string]> = []
  ): PhaseSnapshot => ({
    phase: 'Trick',
    data: {
      to_act: toAct,
      current_trick: currentTrick,
      playable: ['2H', '3H', '5S', '7C'],
      trick_no: 1,
      leader: 0 as Seat,
      last_trick: null,
      round: {
        hand_size: 8,
        leader: 0,
        bid_winner: null,
        trump: null,
        tricks_won: [0, 0, 0, 0],
        bids: [null, null, null, null],
      },
    },
  })

  it('renders empty hand message when hand is empty', () => {
    render(
      <PlayerHand
        viewerHand={[]}
        phase={createTrickPhase(0)}
        playerNames={playerNames}
        viewerSeat={0}
        selectedCard={null}
        onSelectCard={() => {}}
      />
    )

    expect(
      screen.getByText('Hand will appear once available.')
    ).toBeInTheDocument()
  })

  it('renders cards in hand', () => {
    render(
      <PlayerHand
        viewerHand={['2H', '3H', '5S', '7C']}
        phase={createTrickPhase(0)}
        playerNames={playerNames}
        viewerSeat={0}
        selectedCard={null}
        onSelectCard={() => {}}
        playState={{
          viewerSeat: 0,
          playable: ['2H', '3H', '5S', '7C'],
          isPending: false,
          onPlay: async () => {},
        }}
      />
    )

    expect(screen.getByLabelText(/2H/)).toBeInTheDocument()
    expect(screen.getByLabelText(/3H/)).toBeInTheDocument()
    expect(screen.getByLabelText(/5S/)).toBeInTheDocument()
    expect(screen.getByLabelText(/7C/)).toBeInTheDocument()
  })

  it('handles card selection', async () => {
    const onSelectCard = vi.fn()
    const user = userEvent.setup()

    render(
      <PlayerHand
        viewerHand={['2H', '3H', '5S', '7C']}
        phase={createTrickPhase(0)}
        playerNames={playerNames}
        viewerSeat={0}
        selectedCard={null}
        onSelectCard={onSelectCard}
        playState={{
          viewerSeat: 0,
          playable: ['2H', '3H', '5S', '7C'],
          isPending: false,
          onPlay: async () => {},
        }}
      />
    )

    const cardButton = screen.getByLabelText(/2H/)
    await user.click(cardButton)

    expect(onSelectCard).toHaveBeenCalledWith('2H')
  })

  it('shows waiting message in header when not viewer turn and confirmation disabled', () => {
    render(
      <PlayerHand
        viewerHand={['2H', '3H', '5S', '7C']}
        phase={createTrickPhase(1)} // Not viewer's turn (Bailey is seat 1)
        playerNames={playerNames}
        viewerSeat={0}
        selectedCard={null}
        onSelectCard={() => {}}
        requireCardConfirmation={false}
        playState={{
          viewerSeat: 0,
          playable: ['2H', '3H', '5S', '7C'],
          isPending: false,
          onPlay: async () => {},
        }}
      />
    )

    // PlayerHand shows waiting message in header when requireCardConfirmation is false
    // and it's not the viewer's turn
    expect(screen.getByText(/Waiting for Bailey/)).toBeInTheDocument()
  })

  it('shows waiting message in button when not viewer turn and confirmation required', () => {
    render(
      <PlayerHand
        viewerHand={['2H', '3H', '5S', '7C']}
        phase={createTrickPhase(1)} // Not viewer's turn (Bailey is seat 1)
        playerNames={playerNames}
        viewerSeat={0}
        selectedCard={null}
        onSelectCard={() => {}}
        requireCardConfirmation={true} // Default, but explicit for clarity
        playState={{
          viewerSeat: 0,
          playable: ['2H', '3H', '5S', '7C'],
          isPending: false,
          onPlay: async () => {},
        }}
      />
    )

    // PlayerHand button shows waiting message when not viewer's turn and confirmation is required
    // The button text shows "Waiting for Bailey" but aria-label is different, so query by text
    const playButton = screen.getByText(/Waiting for Bailey/)
    expect(playButton).toBeInTheDocument()
    expect(playButton.closest('button')).toBeDisabled()
  })
})
