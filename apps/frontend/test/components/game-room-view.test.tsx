import { describe, expect, it, vi } from 'vitest'
import type { ReactNode } from 'react'
import { fireEvent } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

import { GameRoomView } from '@/app/game/[gameId]/_components/game-room-view'
import {
  biddingSnapshotFixture,
  trickSnapshotFixture,
} from '../mocks/game-snapshot'
import { render, screen } from '../utils'

vi.mock('next/link', () => ({
  __esModule: true,
  default: ({ children, ...props }: { children: ReactNode; href: string }) => (
    <a {...props}>{children}</a>
  ),
}))

describe('GameRoomView', () => {
  const playerNames: [string, string, string, string] = [
    'Alex',
    'Bailey',
    'Casey',
    'Dakota',
  ]

  it('renders phase summary and seat information', () => {
    render(
      <GameRoomView
        gameId={42}
        snapshot={biddingSnapshotFixture}
        playerNames={playerNames}
        viewerSeat={0}
        viewerHand={['2H', '3C']}
        status={{
          lastSyncedAt: new Date('2025-01-06T15:04:05Z').toISOString(),
          isPolling: false,
        }}
        onRefresh={() => undefined}
      />
    )

    expect(screen.getByText('Bidding Round')).toBeInTheDocument()
    expect(screen.getAllByText('Alex').length).toBeGreaterThan(0)
    expect(screen.getAllByText(/Tricks/)[0].textContent).toContain('0')
    expect(screen.getByText('Bid 2')).toBeInTheDocument()
    expect(screen.getByText('Refresh')).toBeInTheDocument()
  })

  it('surface errors and polling status', () => {
    render(
      <GameRoomView
        gameId={42}
        snapshot={biddingSnapshotFixture}
        playerNames={playerNames}
        viewerSeat={0}
        viewerHand={[]}
        status={{ lastSyncedAt: new Date().toISOString(), isPolling: true }}
        error={{ message: 'Sync failed', traceId: 'abc123' }}
      />
    )

    expect(screen.getByText('Sync failed')).toBeInTheDocument()
    expect(screen.getByText(/traceId: abc123/)).toBeInTheDocument()
    expect(screen.getByText('Syncing…')).toBeInTheDocument()
  })

  it('renders bidding controls and submits bid for the active viewer', async () => {
    const onSubmit = vi.fn().mockResolvedValue(undefined)

    render(
      <GameRoomView
        gameId={42}
        snapshot={biddingSnapshotFixture}
        playerNames={playerNames}
        viewerSeat={1}
        viewerHand={[]}
        status={{ lastSyncedAt: new Date().toISOString(), isPolling: false }}
        biddingState={{
          viewerSeat: 1,
          isPending: false,
          onSubmit,
        }}
      />
    )

    const bidInput = screen.getByLabelText('Your Bid') as HTMLInputElement
    expect(bidInput.value).toBe('0')

    fireEvent.change(bidInput, { target: { value: '4' } })
    expect(bidInput.value).toBe('4')

    const submitButton = screen.getByRole('button', { name: 'Submit Bid' })
    expect(submitButton).toBeEnabled()

    await userEvent.click(submitButton)
    expect(onSubmit).toHaveBeenCalledWith(4)
  })

  it('enforces legal card gating and triggers play submission', async () => {
    const onPlay = vi.fn().mockResolvedValue(undefined)

    render(
      <GameRoomView
        gameId={99}
        snapshot={trickSnapshotFixture}
        playerNames={playerNames}
        viewerSeat={0}
        viewerHand={['2H', 'KD', 'QC', 'AS']}
        status={{ lastSyncedAt: new Date().toISOString(), isPolling: false }}
        playState={{
          viewerSeat: 0,
          playable: trickSnapshotFixture.phase.data.playable,
          isPending: false,
          onPlay,
        }}
      />
    )

    expect(screen.getByText('Legal cards: 2H, KD, QC')).toBeInTheDocument()

    const legalCardButton = screen.getByRole('button', { name: '2H' })
    expect(legalCardButton).toBeEnabled()

    const illegalCardButton = screen.getByRole('button', { name: 'AS' })
    expect(illegalCardButton).toBeDisabled()

    await userEvent.click(legalCardButton)
    const playButton = screen.getByRole('button', {
      name: 'Play Selected Card',
    })
    await userEvent.click(playButton)

    expect(onPlay).toHaveBeenCalledWith('2H')
  })

  it('renders AI management panel for host controls', async () => {
    const onAdd = vi.fn()
    const onRemove = vi.fn()

    render(
      <GameRoomView
        gameId={77}
        snapshot={biddingSnapshotFixture}
        playerNames={playerNames}
        viewerSeat={0}
        viewerHand={[]}
        status={{ lastSyncedAt: new Date().toISOString(), isPolling: false }}
        aiSeatState={{
          totalSeats: 4,
          availableSeats: 1,
          aiSeats: 2,
          isPending: false,
          canAdd: true,
          canRemove: true,
          onAdd,
          onRemove,
          seats: [
            {
              seat: 0,
              name: 'Alex',
              playerId: 101,
              isOccupied: true,
              isAi: false,
            },
            {
              seat: 1,
              name: 'Bot Bailey',
              playerId: 202,
              isOccupied: true,
              isAi: true,
            },
            {
              seat: 2,
              name: 'Bot Casey',
              playerId: 303,
              isOccupied: true,
              isAi: true,
            },
            {
              seat: 3,
              name: 'Open',
              playerId: 0,
              isOccupied: false,
              isAi: false,
            },
          ],
        }}
      />
    )

    expect(screen.getByText('AI Seats')).toBeInTheDocument()
    expect(screen.getByText(/2 bots · 3\/4 seats filled/)).toBeInTheDocument()

    const addButton = screen.getByRole('button', { name: 'Add AI' })
    const removeButton = screen.getByRole('button', { name: 'Remove AI' })

    await userEvent.click(addButton)
    await userEvent.click(removeButton)

    expect(onAdd).toHaveBeenCalled()
    expect(onRemove).toHaveBeenCalled()
  })
})
