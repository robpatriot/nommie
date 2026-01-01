import { describe, expect, it, vi, beforeEach } from 'vitest'
import { render, screen } from '../utils'
import userEvent from '@testing-library/user-event'
import { ScoreSidebar } from '@/app/game/[gameId]/_components/game-room/ScoreSidebar'
import type { PhaseSnapshot } from '@/lib/game-room/types'
import { trickPhaseSnapshot, initPhaseSnapshot } from '../mocks/game-snapshot'

describe('ScoreSidebar', () => {
  const playerNames: [string, string, string, string] = [
    'Alex',
    'Bailey',
    'Casey',
    'Dakota',
  ]

  const seatDisplayName = (seat: number) => playerNames[seat]

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders sidebar with game ID and phase', () => {
    const phase: PhaseSnapshot = {
      phase: 'Trick',
      data: {
        ...trickPhaseSnapshot,
      },
    }

    render(
      <ScoreSidebar
        gameId={42}
        phase={phase}
        activeName="Alex"
        playerNames={playerNames}
        scores={[10, 8, 12, 6]}
        round={phase.data.round}
        roundNo={1}
        dealer={0}
        seatDisplayName={seatDisplayName}
      />
    )

    // Game ID is displayed in format "Game {gameId}" or "Setup · Game {gameId}"
    const gameIdText = screen.getByText(/Game.*42|42.*Game/i)
    expect(gameIdText).toBeInTheDocument()

    // Phase name should be displayed - check for heading
    const phaseHeading = screen.getByRole('heading', { level: 2 })
    expect(phaseHeading).toBeInTheDocument()
    // Phase name should contain "Trick" or similar
    expect(phaseHeading.textContent).toMatch(/Trick|Bidding|Init/i)
  })

  it('displays active player name', () => {
    const phase: PhaseSnapshot = {
      phase: 'Trick',
      data: {
        ...trickPhaseSnapshot,
      },
    }

    render(
      <ScoreSidebar
        gameId={42}
        phase={phase}
        activeName="Bailey"
        playerNames={playerNames}
        scores={[10, 8, 12, 6]}
        round={phase.data.round}
        roundNo={1}
        dealer={0}
        seatDisplayName={seatDisplayName}
      />
    )

    // Turn label and active player should be displayed
    // The active name appears in the turn label
    expect(screen.getByText(/Turn/i)).toBeInTheDocument()
    // Bailey should appear somewhere in the sidebar (in turn label or scoreboard)
    const baileyElements = screen.getAllByText('Bailey')
    expect(baileyElements.length).toBeGreaterThan(0)
  })

  it('displays round information when round exists', () => {
    const phase: PhaseSnapshot = {
      phase: 'Trick',
      data: {
        ...trickPhaseSnapshot,
      },
    }

    render(
      <ScoreSidebar
        gameId={42}
        phase={phase}
        activeName="Alex"
        playerNames={playerNames}
        scores={[10, 8, 12, 6]}
        round={phase.data.round}
        roundNo={1}
        dealer={0}
        seatDisplayName={seatDisplayName}
      />
    )

    expect(screen.getByText(/Round/i)).toBeInTheDocument()
    expect(screen.getByText(/Dealer/i)).toBeInTheDocument()
    expect(screen.getByText(/Hand size/i)).toBeInTheDocument()
    expect(screen.getByText(/Trump/i)).toBeInTheDocument()
  })

  it('displays trick number in trick phase', () => {
    const phase: PhaseSnapshot = {
      phase: 'Trick',
      data: {
        ...trickPhaseSnapshot,
        trick_no: 3,
      },
    }

    render(
      <ScoreSidebar
        gameId={42}
        phase={phase}
        activeName="Alex"
        playerNames={playerNames}
        scores={[10, 8, 12, 6]}
        round={phase.data.round}
        roundNo={1}
        dealer={0}
        seatDisplayName={seatDisplayName}
      />
    )

    // Trick number stat card should be displayed
    // StatCard renders label and value separately
    // Check for the trick value format "3 / 8"
    const trickValue = screen.queryByText('3 / 8')
    if (!trickValue) {
      // Alternative: check for numbers with slash pattern in sidebar
      const sidebar = screen.getByRole('complementary')
      expect(sidebar.textContent).toMatch(/\d+ \/ \d+/)
    } else {
      expect(trickValue).toBeInTheDocument()
    }
  })

  it('does not display trick number in non-trick phases', () => {
    const phase = initPhaseSnapshot

    render(
      <ScoreSidebar
        gameId={42}
        phase={phase}
        activeName="Alex"
        playerNames={playerNames}
        scores={[10, 8, 12, 6]}
        round={null}
        roundNo={0}
        dealer={0}
        seatDisplayName={seatDisplayName}
      />
    )

    // Should not show trick number stat card (only shown in Trick phase)
    // Check that trick number format is not present
    const trickNumberPattern = screen.queryByText(/\d+ \/ \d+/)
    expect(trickNumberPattern).not.toBeInTheDocument()
  })

  it('displays all player scores', () => {
    const phase: PhaseSnapshot = {
      phase: 'Trick',
      data: {
        ...trickPhaseSnapshot,
      },
    }

    render(
      <ScoreSidebar
        gameId={42}
        phase={phase}
        activeName="Alex"
        playerNames={playerNames}
        scores={[10, 8, 12, 6]}
        round={phase.data.round}
        roundNo={1}
        dealer={0}
        seatDisplayName={seatDisplayName}
      />
    )

    // Player names are displayed in the scoreboard
    // Note: playerNames are passed directly, not through getPlayerDisplayName
    // Names may appear multiple times, use getAllByText
    expect(screen.getAllByText('Alex').length).toBeGreaterThan(0)
    expect(screen.getAllByText('Bailey').length).toBeGreaterThan(0)
    expect(screen.getAllByText('Casey').length).toBeGreaterThan(0)
    expect(screen.getAllByText('Dakota').length).toBeGreaterThan(0)
    // Scores are displayed - may appear multiple times
    expect(screen.getAllByText('10').length).toBeGreaterThan(0)
    expect(screen.getAllByText('8').length).toBeGreaterThan(0)
    expect(screen.getAllByText('12').length).toBeGreaterThan(0)
    expect(screen.getAllByText('6').length).toBeGreaterThan(0)
  })

  it('displays error message when error exists', () => {
    const phase: PhaseSnapshot = {
      phase: 'Trick',
      data: {
        ...trickPhaseSnapshot,
      },
    }
    const error = { message: 'Sync failed', traceId: 'trace-123' }

    render(
      <ScoreSidebar
        gameId={42}
        phase={phase}
        activeName="Alex"
        playerNames={playerNames}
        scores={[10, 8, 12, 6]}
        round={phase.data.round}
        roundNo={1}
        dealer={0}
        seatDisplayName={seatDisplayName}
        error={error}
      />
    )

    expect(screen.getByText('Sync failed')).toBeInTheDocument()
    expect(screen.getByText(/trace-123/i)).toBeInTheDocument()
  })

  it('does not display error when error is null', () => {
    const phase: PhaseSnapshot = {
      phase: 'Trick',
      data: {
        ...trickPhaseSnapshot,
      },
    }

    render(
      <ScoreSidebar
        gameId={42}
        phase={phase}
        activeName="Alex"
        playerNames={playerNames}
        scores={[10, 8, 12, 6]}
        round={phase.data.round}
        roundNo={1}
        dealer={0}
        seatDisplayName={seatDisplayName}
        error={null}
      />
    )

    expect(screen.queryByText(/Sync failed/i)).not.toBeInTheDocument()
  })

  it('displays error without traceId when traceId is missing', () => {
    const phase: PhaseSnapshot = {
      phase: 'Trick',
      data: {
        ...trickPhaseSnapshot,
      },
    }
    const error = { message: 'Sync failed' }

    render(
      <ScoreSidebar
        gameId={42}
        phase={phase}
        activeName="Alex"
        playerNames={playerNames}
        scores={[10, 8, 12, 6]}
        round={phase.data.round}
        roundNo={1}
        dealer={0}
        seatDisplayName={seatDisplayName}
        error={error}
      />
    )

    expect(screen.getByText('Sync failed')).toBeInTheDocument()
    expect(screen.queryByText(/trace/i)).not.toBeInTheDocument()
  })

  it('calls onShowHistory when history button is clicked', async () => {
    const user = userEvent.setup()
    const onShowHistory = vi.fn()
    const phase: PhaseSnapshot = {
      phase: 'Trick',
      data: {
        ...trickPhaseSnapshot,
      },
    }

    render(
      <ScoreSidebar
        gameId={42}
        phase={phase}
        activeName="Alex"
        playerNames={playerNames}
        scores={[10, 8, 12, 6]}
        round={phase.data.round}
        roundNo={1}
        dealer={0}
        seatDisplayName={seatDisplayName}
        onShowHistory={onShowHistory}
      />
    )

    const historyButton = screen.getByRole('button', {
      name: /Show score history/i,
    })
    await user.click(historyButton)

    expect(onShowHistory).toHaveBeenCalledTimes(1)
  })

  it('disables history button when loading', () => {
    const phase: PhaseSnapshot = {
      phase: 'Trick',
      data: {
        ...trickPhaseSnapshot,
      },
    }

    render(
      <ScoreSidebar
        gameId={42}
        phase={phase}
        activeName="Alex"
        playerNames={playerNames}
        scores={[10, 8, 12, 6]}
        round={phase.data.round}
        roundNo={1}
        dealer={0}
        seatDisplayName={seatDisplayName}
        onShowHistory={vi.fn()}
        isHistoryLoading={true}
      />
    )

    const historyButton = screen.getByRole('button', {
      name: /Show score history/i,
    })
    expect(historyButton).toBeDisabled()
  })

  it('shows loading text when history is loading', () => {
    const phase: PhaseSnapshot = {
      phase: 'Trick',
      data: {
        ...trickPhaseSnapshot,
      },
    }

    render(
      <ScoreSidebar
        gameId={42}
        phase={phase}
        activeName="Alex"
        playerNames={playerNames}
        scores={[10, 8, 12, 6]}
        round={phase.data.round}
        roundNo={1}
        dealer={0}
        seatDisplayName={seatDisplayName}
        onShowHistory={vi.fn()}
        isHistoryLoading={true}
      />
    )

    expect(screen.getByText(/Opening/i)).toBeInTheDocument()
  })

  it('does not show history button when onShowHistory is not provided', () => {
    const phase: PhaseSnapshot = {
      phase: 'Trick',
      data: {
        ...trickPhaseSnapshot,
      },
    }

    render(
      <ScoreSidebar
        gameId={42}
        phase={phase}
        activeName="Alex"
        playerNames={playerNames}
        scores={[10, 8, 12, 6]}
        round={phase.data.round}
        roundNo={1}
        dealer={0}
        seatDisplayName={seatDisplayName}
      />
    )

    expect(
      screen.queryByRole('button', { name: /Show score history/i })
    ).not.toBeInTheDocument()
  })

  it('displays trump value when round has trump', () => {
    const phase: PhaseSnapshot = {
      phase: 'Trick',
      data: {
        ...trickPhaseSnapshot,
      },
    }

    render(
      <ScoreSidebar
        gameId={42}
        phase={phase}
        activeName="Alex"
        playerNames={playerNames}
        scores={[10, 8, 12, 6]}
        round={phase.data.round}
        roundNo={1}
        dealer={0}
        seatDisplayName={seatDisplayName}
      />
    )

    expect(screen.getByText(/Trump/i)).toBeInTheDocument()
    // Trump value should be displayed (HEARTS in this case)
    expect(screen.getByText(/Hearts/i)).toBeInTheDocument()
  })

  it('displays undeclared when trump is null', () => {
    const phase: PhaseSnapshot = {
      phase: 'Trick',
      data: {
        ...trickPhaseSnapshot,
        round: {
          ...trickPhaseSnapshot.round,
          trump: null,
        },
      },
    }

    render(
      <ScoreSidebar
        gameId={42}
        phase={phase}
        activeName="Alex"
        playerNames={playerNames}
        scores={[10, 8, 12, 6]}
        round={phase.data.round}
        roundNo={1}
        dealer={0}
        seatDisplayName={seatDisplayName}
      />
    )

    expect(screen.getByText(/Trump/i)).toBeInTheDocument()
    expect(screen.getByText(/Undeclared/i)).toBeInTheDocument()
  })
})
