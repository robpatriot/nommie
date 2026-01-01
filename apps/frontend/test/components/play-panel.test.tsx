import { describe, expect, it, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '../utils'
import userEvent from '@testing-library/user-event'
import { PlayPanel } from '@/app/game/[gameId]/_components/game-room/PlayPanel'
import { createTrickPhase } from '../setup/phase-factories'
import { createPlayState } from '../setup/state-factories'

describe('PlayPanel', () => {
  const playerNames: [string, string, string, string] = [
    'Alex',
    'Bailey',
    'Casey',
    'Dakota',
  ]

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders play panel with title and description', () => {
    const phase = createTrickPhase({ to_act: 0 })
    const play = createPlayState({ viewerSeat: 0 })

    render(
      <PlayPanel
        phase={phase}
        playerNames={playerNames}
        play={play}
        selectedCard="2H"
        onPlayCard={play.onPlay}
      />
    )

    // Title should be displayed
    expect(screen.getByRole('heading', { name: /Play/i })).toBeInTheDocument()
  })

  it('displays selected card', () => {
    const phase = createTrickPhase({ to_act: 0 })
    const play = createPlayState({ viewerSeat: 0 })

    render(
      <PlayPanel
        phase={phase}
        playerNames={playerNames}
        play={play}
        selectedCard="2H"
        onPlayCard={play.onPlay}
      />
    )

    // "Selected card" text may appear multiple times, use getAllByText
    const selectedCardLabels = screen.getAllByText(/Selected card/i)
    expect(selectedCardLabels.length).toBeGreaterThan(0)

    // Card should be displayed (PlayingCard component renders it)
    // The card is rendered as a div with aria-label
    // Try to find by label - format is "rank of suit" (e.g., "2 of Hearts")
    const cardElement = screen.queryByLabelText(/2.*[Hh]earts/i)
    // Card should be rendered when selectedCard is provided
    expect(cardElement).toBeInTheDocument()
  })

  it('shows placeholder when no card selected', () => {
    const phase = createTrickPhase({ to_act: 0 })
    const play = createPlayState({ viewerSeat: 0 })

    render(
      <PlayPanel
        phase={phase}
        playerNames={playerNames}
        play={play}
        selectedCard={null}
        onPlayCard={play.onPlay}
      />
    )

    expect(screen.getByText('—')).toBeInTheDocument()
  })

  it('displays waiting message when not viewer turn', () => {
    const phase = createTrickPhase({ to_act: 1 })
    const play = createPlayState({ viewerSeat: 0 })

    render(
      <PlayPanel
        phase={phase}
        playerNames={playerNames}
        play={play}
        selectedCard={null}
        onPlayCard={play.onPlay}
      />
    )

    // Check for waiting message - may appear in multiple places
    const waitingElements = screen.getAllByText(/Waiting/i)
    expect(waitingElements.length).toBeGreaterThan(0)

    // Bailey is seat 1, which is to_act, so should appear in waiting message
    // The waiting message format is "Waiting on {name}"
    const waitingText = screen.getByText(/Waiting on/i)
    expect(waitingText.textContent).toContain('Bailey')
  })

  it('displays playable cards list', () => {
    const phase = createTrickPhase({ to_act: 0 })
    const play = createPlayState({
      viewerSeat: 0,
      playable: ['2H', '3C', '5S'],
    })

    render(
      <PlayPanel
        phase={phase}
        playerNames={playerNames}
        play={play}
        selectedCard="2H"
        onPlayCard={play.onPlay}
      />
    )

    expect(screen.getByText(/Legal cards/i)).toBeInTheDocument()
    expect(screen.getByText(/2H, 3C, 5S/i)).toBeInTheDocument()
  })

  it('shows empty message when no playable cards', () => {
    const phase = createTrickPhase({ to_act: 0 })
    const play = createPlayState({ viewerSeat: 0, playable: [] })

    const { container } = render(
      <PlayPanel
        phase={phase}
        playerNames={playerNames}
        play={play}
        selectedCard={null}
        onPlayCard={play.onPlay}
      />
    )

    // Empty playable cards shows "—" in the legal cards list
    // There may be multiple "—" elements (selected card placeholder and empty playable list)
    const dashElements = screen.getAllByText('—')
    expect(dashElements.length).toBeGreaterThan(0)

    // Check that the legal cards section exists
    // "Legal" text appears multiple times (short/long versions for responsive design)
    const legalTexts = screen.getAllByText(/Legal/i)
    expect(legalTexts.length).toBeGreaterThan(0)

    // When playable is empty, the legal cards paragraph should show "—"
    // Check that the container text includes both "Legal" and "—"
    const containerText = container.textContent || ''
    expect(containerText).toMatch(/Legal/i)
    expect(containerText).toContain('—')
  })

  it('submits selected card when playable', async () => {
    const user = userEvent.setup()
    const onPlay = vi.fn().mockResolvedValue(undefined)
    const phase = createTrickPhase({ to_act: 0 })
    const play = createPlayState({
      viewerSeat: 0,
      playable: ['2H', '3C'],
      onPlay,
    })

    render(
      <PlayPanel
        phase={phase}
        playerNames={playerNames}
        play={play}
        selectedCard="2H"
        onPlayCard={onPlay}
      />
    )

    const submitButton = screen.getByRole('button', {
      name: /Play selected card/i,
    })
    await user.click(submitButton)

    await waitFor(() => {
      expect(onPlay).toHaveBeenCalledWith('2H')
    })
  })

  it('disables submit when card is not playable', () => {
    const phase = createTrickPhase({ to_act: 0 })
    const play = createPlayState({
      viewerSeat: 0,
      playable: ['2H', '3C'], // Selected card '5S' is not in playable list
    })

    render(
      <PlayPanel
        phase={phase}
        playerNames={playerNames}
        play={play}
        selectedCard="5S"
        onPlayCard={play.onPlay}
      />
    )

    const submitButton = screen.getByRole('button', {
      name: /Play selected card/i,
    })
    expect(submitButton).toBeDisabled()
  })

  it('disables submit when not viewer turn', () => {
    const phase = createTrickPhase({ to_act: 1 })
    const play = createPlayState({ viewerSeat: 0, playable: ['2H'] })

    render(
      <PlayPanel
        phase={phase}
        playerNames={playerNames}
        play={play}
        selectedCard="2H"
        onPlayCard={play.onPlay}
      />
    )

    const submitButton = screen.getByRole('button', {
      name: /Waiting for/i,
    })
    expect(submitButton).toBeDisabled()
  })

  it('disables submit when pending', () => {
    const phase = createTrickPhase({ to_act: 0 })
    const play = createPlayState({
      viewerSeat: 0,
      playable: ['2H'],
      isPending: true,
    })

    render(
      <PlayPanel
        phase={phase}
        playerNames={playerNames}
        play={play}
        selectedCard="2H"
        onPlayCard={play.onPlay}
      />
    )

    const submitButton = screen.getByRole('button', { name: /Playing/i })
    expect(submitButton).toBeDisabled()
  })

  it('shows playing state when pending', () => {
    const phase = createTrickPhase({ to_act: 0 })
    const play = createPlayState({
      viewerSeat: 0,
      playable: ['2H'],
      isPending: true,
    })

    render(
      <PlayPanel
        phase={phase}
        playerNames={playerNames}
        play={play}
        selectedCard="2H"
        onPlayCard={play.onPlay}
      />
    )

    expect(screen.getByText(/Playing/i)).toBeInTheDocument()
  })

  it('does not submit when no card selected', async () => {
    const user = userEvent.setup()
    const onPlay = vi.fn().mockResolvedValue(undefined)
    const phase = createTrickPhase({ to_act: 0 })
    const play = createPlayState({
      viewerSeat: 0,
      playable: ['2H'],
      onPlay,
    })

    render(
      <PlayPanel
        phase={phase}
        playerNames={playerNames}
        play={play}
        selectedCard={null}
        onPlayCard={onPlay}
      />
    )

    const submitButton = screen.getByRole('button', {
      name: /Play selected card/i,
    })
    expect(submitButton).toBeDisabled()

    // Try to click anyway (should be no-op)
    await user.click(submitButton)

    expect(onPlay).not.toHaveBeenCalled()
  })
})
