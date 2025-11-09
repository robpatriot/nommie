import { describe, expect, it, vi } from 'vitest'
import type { ReactNode } from 'react'
import { fireEvent } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

import { GameRoomView } from '@/app/game/[gameId]/_components/game-room-view'
import { biddingSnapshotFixture } from '../mocks/game-snapshot'
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
})
