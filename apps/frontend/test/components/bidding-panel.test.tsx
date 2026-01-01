import { describe, expect, it, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '../utils'
import userEvent from '@testing-library/user-event'
import { BiddingPanel } from '@/app/game/[gameId]/_components/game-room/BiddingPanel'
import { createBiddingPhase, createTrumpPhase } from '../setup/phase-factories'
import { createBiddingState, createTrumpState } from '../setup/state-factories'

// Mock useMediaQuery hook
vi.mock('@/hooks/useMediaQuery', () => ({
  useMediaQuery: (_query: string) => {
    return true
  },
}))

describe('BiddingPanel', () => {
  const playerNames: [string, string, string, string] = [
    'Alex',
    'Bailey',
    'Casey',
    'Dakota',
  ]

  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('Bidding Mode', () => {
    it('renders bidding panel with title and subtitle', () => {
      const phase = createBiddingPhase({ to_act: 0 })
      const bidding = createBiddingState({ viewerSeat: 0 })

      render(
        <BiddingPanel
          phase={phase}
          viewerSeat={0}
          layoutSeat={0}
          playerNames={playerNames}
          bidding={bidding}
        />
      )

      expect(screen.getByText(/Bidding/i)).toBeInTheDocument()
      expect(screen.getByText(/Your bid/i)).toBeInTheDocument()
    })

    it('shows your turn badge when it is viewer turn', () => {
      const phase = createBiddingPhase({ to_act: 0 })
      const bidding = createBiddingState({ viewerSeat: 0 })

      render(
        <BiddingPanel
          phase={phase}
          viewerSeat={0}
          layoutSeat={0}
          playerNames={playerNames}
          bidding={bidding}
        />
      )

      expect(screen.getByText(/Your turn/i)).toBeInTheDocument()
    })

    it('shows waiting badge when it is not viewer turn', () => {
      const phase = createBiddingPhase({ to_act: 1 })
      const bidding = createBiddingState({ viewerSeat: 0 })

      render(
        <BiddingPanel
          phase={phase}
          viewerSeat={0}
          layoutSeat={0}
          playerNames={playerNames}
          bidding={bidding}
        />
      )

      // Check for waiting badge - may appear multiple times
      const waitingElements = screen.getAllByText(/Waiting/i)
      expect(waitingElements.length).toBeGreaterThan(0)
      // Bailey name may appear multiple times, use getAllByText
      const baileyElements = screen.getAllByText('Bailey')
      expect(baileyElements.length).toBeGreaterThan(0)
    })

    it('allows entering a bid value', async () => {
      const user = userEvent.setup()
      const phase = createBiddingPhase({ to_act: 0 })
      const bidding = createBiddingState({ viewerSeat: 0 })

      render(
        <BiddingPanel
          phase={phase}
          viewerSeat={0}
          layoutSeat={0}
          playerNames={playerNames}
          bidding={bidding}
        />
      )

      const input = screen.getByLabelText(/Bid value/i) as HTMLInputElement
      await user.type(input, '5')

      expect(input.value).toBe('5')
    })

    it('validates minimum bid', async () => {
      const user = userEvent.setup()
      const phase = createBiddingPhase({ to_act: 0, min_bid: 2 })
      const bidding = createBiddingState({ viewerSeat: 0 })

      render(
        <BiddingPanel
          phase={phase}
          viewerSeat={0}
          layoutSeat={0}
          playerNames={playerNames}
          bidding={bidding}
        />
      )

      const input = screen.getByLabelText(/Bid value/i) as HTMLInputElement
      await user.type(input, '1')

      // Input should accept the value
      expect(input.value).toBe('1')

      // Submit button should be enabled (validation happens on submit)
      const submitButton = screen.getByRole('button', { name: /Submit/i })
      expect(submitButton).not.toBeDisabled()

      // Click submit - component normalizes invalid bids to valid range
      await user.click(submitButton)

      // Component normalizes bid to min_bid (2) when value is too low
      await waitFor(() => {
        expect(bidding.onSubmit).toHaveBeenCalledWith(2)
      })
    })

    it('validates maximum bid', async () => {
      const user = userEvent.setup()
      const phase = createBiddingPhase({ to_act: 0, max_bid: 8 })
      const bidding = createBiddingState({ viewerSeat: 0 })

      render(
        <BiddingPanel
          phase={phase}
          viewerSeat={0}
          layoutSeat={0}
          playerNames={playerNames}
          bidding={bidding}
        />
      )

      const input = screen.getByLabelText(/Bid value/i) as HTMLInputElement
      await user.type(input, '10')

      // Input should accept the value
      expect(input.value).toBe('10')

      // Submit button should be enabled
      const submitButton = screen.getByRole('button', { name: /Submit/i })
      expect(submitButton).not.toBeDisabled()

      // Click submit - component normalizes invalid bids to valid range
      await user.click(submitButton)

      // Component normalizes bid to max_bid (8) when value is too high
      await waitFor(() => {
        expect(bidding.onSubmit).toHaveBeenCalledWith(8)
      })
    })

    it('validates zero bid when locked', async () => {
      const user = userEvent.setup()
      const phase = createBiddingPhase({ to_act: 0 })
      const bidding = createBiddingState({
        viewerSeat: 0,
        zeroBidLocked: true,
      })

      render(
        <BiddingPanel
          phase={phase}
          viewerSeat={0}
          layoutSeat={0}
          playerNames={playerNames}
          bidding={bidding}
        />
      )

      const input = screen.getByLabelText(/Bid value/i)
      await user.type(input, '0')
      const submitButton = screen.getByRole('button', { name: /Submit/i })

      await user.click(submitButton)

      // Validation message appears in role="alert" element after submit
      await waitFor(() => {
        const alert = screen.getByRole('alert')
        expect(alert).toHaveTextContent(/maximum number of times/i)
      })
    })

    it('validates total cannot equal hand size on final bid', async () => {
      const user = userEvent.setup()
      // Set up as final bid (only one null bid remaining) so validation triggers
      // Viewer is at seat 1, so bids[1] must be null for them to bid
      const phase = createBiddingPhase({
        to_act: 1,
        bids: [2, null, 3, 2] as [
          number | null,
          number | null,
          number | null,
          number | null,
        ],
        round: {
          hand_size: 8,
          leader: 0,
          bid_winner: null,
          trump: null,
          tricks_won: [0, 0, 0, 0],
          bids: [2, null, 3, 2] as [
            number | null,
            number | null,
            number | null,
            number | null,
          ],
        },
      })
      const bidding = createBiddingState({ viewerSeat: 1 })

      render(
        <BiddingPanel
          phase={phase}
          viewerSeat={1}
          layoutSeat={1}
          playerNames={playerNames}
          bidding={bidding}
        />
      )

      const input = screen.getByLabelText(/Bid value/i) as HTMLInputElement
      // 2 + 1 + 3 + 2 = 8 (hand size) - this should trigger validation
      await user.type(input, '1')

      // Input should accept the value
      expect(input.value).toBe('1')

      // Submit button should be enabled
      const submitButton = screen.getByRole('button', { name: /Submit/i })
      expect(submitButton).not.toBeDisabled()

      // Click submit - validation should show warning and prevent submission
      // Validation only applies when this is the final bid (remainingNullBids === 1)
      await user.click(submitButton)

      // Validation warning should appear and prevent submission
      await waitFor(() => {
        const alert = screen.getByRole('alert')
        expect(alert).toHaveTextContent(/Total bids cannot equal/i)
      })
      expect(bidding.onSubmit).not.toHaveBeenCalled()
    })

    it('submits valid bid', async () => {
      const user = userEvent.setup()
      const onSubmit = vi.fn().mockResolvedValue(undefined)
      const phase = createBiddingPhase({ to_act: 0 })
      const bidding = createBiddingState({ viewerSeat: 0, onSubmit })

      render(
        <BiddingPanel
          phase={phase}
          viewerSeat={0}
          layoutSeat={0}
          playerNames={playerNames}
          bidding={bidding}
        />
      )

      const input = screen.getByLabelText(/Bid value/i)
      await user.type(input, '5')
      const submitButton = screen.getByRole('button', { name: /Submit/i })

      await user.click(submitButton)

      await waitFor(() => {
        expect(onSubmit).toHaveBeenCalledWith(5)
      })
    })

    it('constrains input to valid range on blur', async () => {
      const user = userEvent.setup()
      const phase = createBiddingPhase({ to_act: 0, min_bid: 0, max_bid: 8 })
      const bidding = createBiddingState({ viewerSeat: 0 })

      render(
        <BiddingPanel
          phase={phase}
          viewerSeat={0}
          layoutSeat={0}
          playerNames={playerNames}
          bidding={bidding}
        />
      )

      const input = screen.getByLabelText(/Bid value/i) as HTMLInputElement
      await user.type(input, '15')
      await user.tab() // Blur the input

      await waitFor(() => {
        expect(input.value).toBe('8') // Constrained to max
      })
    })

    it('disables submit when pending', () => {
      const phase = createBiddingPhase({ to_act: 0 })
      const bidding = createBiddingState({ viewerSeat: 0, isPending: true })

      render(
        <BiddingPanel
          phase={phase}
          viewerSeat={0}
          layoutSeat={0}
          playerNames={playerNames}
          bidding={bidding}
        />
      )

      const submitButton = screen.getByRole('button', { name: /Submitting/i })
      expect(submitButton).toBeDisabled()
    })

    it('hides form when viewer has already bid', () => {
      const phase = createBiddingPhase({
        to_act: 1,
        bids: [5, null, null, null] as [
          number | null,
          number | null,
          number | null,
          number | null,
        ],
      })
      const bidding = createBiddingState({ viewerSeat: 0 })

      render(
        <BiddingPanel
          phase={phase}
          viewerSeat={0}
          layoutSeat={0}
          playerNames={playerNames}
          bidding={bidding}
        />
      )

      expect(screen.queryByLabelText(/Bid value/i)).not.toBeInTheDocument()
    })

    it('displays all player bids in table', () => {
      const phase = createBiddingPhase({
        bids: [2, 3, null, 1] as [
          number | null,
          number | null,
          number | null,
          number | null,
        ],
      })
      const bidding = createBiddingState({ viewerSeat: 0 })

      render(
        <BiddingPanel
          phase={phase}
          viewerSeat={0}
          layoutSeat={0}
          playerNames={playerNames}
          bidding={bidding}
        />
      )

      // Viewer seat (0) shows "You", others show their names
      // Player names may appear multiple times, use getAllByText
      expect(screen.getAllByText(/You/i).length).toBeGreaterThan(0)
      const baileyElements = screen.getAllByText('Bailey')
      expect(baileyElements.length).toBeGreaterThan(0)
      expect(screen.getAllByText('Casey').length).toBeGreaterThan(0)
      expect(screen.getAllByText('Dakota').length).toBeGreaterThan(0)
      // Bids are displayed
      expect(screen.getByText('2')).toBeInTheDocument()
      expect(screen.getByText('3')).toBeInTheDocument()
      expect(screen.getByText('1')).toBeInTheDocument()
      expect(screen.getAllByText('—').length).toBeGreaterThan(0) // Null bid
    })

    it('highlights active player in bids table', () => {
      const phase = createBiddingPhase({ to_act: 1 })
      const bidding = createBiddingState({ viewerSeat: 0 })

      render(
        <BiddingPanel
          phase={phase}
          viewerSeat={0}
          layoutSeat={0}
          playerNames={playerNames}
          bidding={bidding}
        />
      )

      // Active player should be highlighted - Bailey is seat 1, which is to_act
      const baileyElements = screen.getAllByText('Bailey')
      const baileyElement = baileyElements[0].closest('div')
      expect(baileyElement).toHaveClass(/bg-panel-primary/)
    })
  })

  describe('Trump Selection Mode', () => {
    it('renders trump selection panel', () => {
      const phase = createBiddingPhase({ to_act: 0 })
      const trumpPhase = createTrumpPhase({ to_act: 0 })
      const bidding = createBiddingState({ viewerSeat: 0 })
      const trump = createTrumpState({ canSelect: true })

      render(
        <BiddingPanel
          phase={phase}
          viewerSeat={0}
          layoutSeat={0}
          playerNames={playerNames}
          bidding={bidding}
          trumpPhase={trumpPhase}
          trump={trump}
        />
      )

      expect(screen.getByText(/Select trump/i)).toBeInTheDocument()
    })

    it('displays available trump suits', () => {
      const phase = createBiddingPhase({ to_act: 0 })
      const trumpPhase = createTrumpPhase({
        to_act: 0,
        allowed_trumps: ['HEARTS', 'SPADES', 'NO_TRUMPS'],
      })
      const bidding = createBiddingState({ viewerSeat: 0 })
      const trump = createTrumpState({ canSelect: true })

      render(
        <BiddingPanel
          phase={phase}
          viewerSeat={0}
          layoutSeat={0}
          playerNames={playerNames}
          bidding={bidding}
          trumpPhase={trumpPhase}
          trump={trump}
        />
      )

      // Check for suit symbols - they're rendered as buttons with aria-labels
      const heartsButton = screen.getByRole('button', {
        name: /HEARTS|Hearts/i,
      })
      expect(heartsButton).toBeInTheDocument()
      const spadesButton = screen.getByRole('button', {
        name: /SPADES|Spades/i,
      })
      expect(spadesButton).toBeInTheDocument()
      const noTrumpsButton = screen.getByRole('button', {
        name: /NO_TRUMPS|No trumps/i,
      })
      expect(noTrumpsButton).toBeInTheDocument()
    })

    it('allows selecting a trump suit', async () => {
      const user = userEvent.setup()
      const onSelect = vi.fn().mockResolvedValue(undefined)
      const phase = createBiddingPhase({ to_act: 0 })
      const trumpPhase = createTrumpPhase({ to_act: 0 })
      const bidding = createBiddingState({ viewerSeat: 0 })
      const trump = createTrumpState({ canSelect: true, onSelect })

      render(
        <BiddingPanel
          phase={phase}
          viewerSeat={0}
          layoutSeat={0}
          playerNames={playerNames}
          bidding={bidding}
          trumpPhase={trumpPhase}
          trump={trump}
        />
      )

      // Find and click HEARTS button
      const heartsButton = screen
        .getAllByRole('button')
        .find((btn) => btn.textContent?.includes('♥'))

      expect(heartsButton).toBeDefined()
      await user.click(heartsButton!)

      const submitButton = screen.getByRole('button', {
        name: /Confirm/i,
      })
      await user.click(submitButton)

      await waitFor(() => {
        expect(onSelect).toHaveBeenCalledWith('HEARTS')
      })
    })

    it('disables trump selection when not viewer turn', () => {
      const phase = createBiddingPhase({ to_act: 0 })
      const trumpPhase = createTrumpPhase({ to_act: 1 })
      const bidding = createBiddingState({ viewerSeat: 0 })
      const trump = createTrumpState({ canSelect: false })

      render(
        <BiddingPanel
          phase={phase}
          viewerSeat={0}
          layoutSeat={0}
          playerNames={playerNames}
          bidding={bidding}
          trumpPhase={trumpPhase}
          trump={trump}
        />
      )

      // When canSelect is false, the form should not be rendered
      // Check that the panel shows waiting message instead of form
      expect(screen.getByText(/Select trumps/i)).toBeInTheDocument()

      // The form should not be rendered when canSelect is false
      // (form only renders when canSelectTrump is true in trump mode)
      expect(screen.queryByRole('form')).not.toBeInTheDocument()
    })

    it('disables submit when no trump selected', () => {
      const phase = createBiddingPhase({ to_act: 0 })
      const trumpPhase = createTrumpPhase({ to_act: 0 })
      const bidding = createBiddingState({ viewerSeat: 0 })
      const trump = createTrumpState({ canSelect: true })

      render(
        <BiddingPanel
          phase={phase}
          viewerSeat={0}
          layoutSeat={0}
          playerNames={playerNames}
          bidding={bidding}
          trumpPhase={trumpPhase}
          trump={trump}
        />
      )

      // Submit button should be disabled when no trump is selected
      // Find submit button by looking for the one with type="submit"
      const submitButtons = screen.getAllByRole('button')
      const submitButton = submitButtons.find(
        (btn) => (btn as HTMLButtonElement).type === 'submit'
      )
      expect(submitButton).toBeDefined()
      expect(submitButton).toBeDisabled()
    })
  })
})
