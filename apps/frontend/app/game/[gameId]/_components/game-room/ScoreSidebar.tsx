import type { PhaseSnapshot, RoundPublic, Seat } from '@/lib/game-room/types'
import { formatTrump, getPhaseLabel } from './utils'
import { PhaseFact } from './PhaseFact'
import { StatCard } from '@/components/StatCard'

interface ScoreSidebarProps {
  gameId: number
  phase: PhaseSnapshot
  activeName: string
  playerNames: [string, string, string, string]
  scores: [number, number, number, number]
  round: RoundPublic | null
  roundNo: number
  dealer: Seat
  seatDisplayName: (seat: Seat) => string
  error?: { message: string; traceId?: string } | null
  onRefresh?: () => void
  isRefreshing?: boolean
  className?: string
}

export function ScoreSidebar({
  gameId,
  phase,
  activeName,
  playerNames,
  scores,
  round,
  roundNo,
  dealer,
  seatDisplayName,
  error,
  onRefresh,
  isRefreshing = false,
  className = '',
}: ScoreSidebarProps) {
  return (
    <aside
      className={`flex h-full flex-col gap-4 rounded-3xl border border-white/10 bg-surface/85 p-5 shadow-[0_25px_80px_rgba(0,0,0,0.35)] backdrop-blur ${className}`}
    >
      <header className="grid grid-cols-[1fr_auto] gap-x-3 gap-y-2">
        <div>
          <p className="mb-1 text-[10px] font-semibold uppercase tracking-[0.4em] text-subtle">
            Game #{gameId}
          </p>
          <h2 className="text-2xl font-bold text-foreground">
            {getPhaseLabel(phase)}
          </h2>
        </div>
        {phase.phase === 'Trick' ? (
          <StatCard
            label="Trick"
            value={`${phase.data.trick_no} / ${round?.hand_size ?? '?'}`}
            description="Round progress"
            className="px-2 py-1.5 row-span-2 self-start"
            valueClassName="text-lg"
          />
        ) : null}
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-2 rounded-lg bg-primary/15 px-3 py-1.5">
            <span className="text-[10px] font-semibold uppercase tracking-[0.3em] text-subtle">
              Turn
            </span>
            <span className="text-sm font-bold text-primary">{activeName}</span>
          </div>
          {onRefresh ? (
            <button
              type="button"
              onClick={onRefresh}
              disabled={isRefreshing}
              className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-white/20 bg-surface/60 text-xs font-semibold text-foreground transition hover:border-primary/60 hover:bg-primary/10 hover:text-primary disabled:opacity-60"
              aria-label="Refresh game state"
            >
              {isRefreshing ? (
                <span className="animate-spin">⟳</span>
              ) : (
                <span>⟳</span>
              )}
            </button>
          ) : null}
        </div>
      </header>

      {round ? (
        <div className="grid grid-cols-2 gap-x-4 gap-y-3 rounded-2xl border border-border/60 bg-surface/70 p-3 text-sm text-muted">
          <PhaseFact label="Round" value={`#${roundNo}`} />
          <PhaseFact label="Dealer" value={seatDisplayName(dealer)} />
          <PhaseFact label="Hand Size" value={round.hand_size.toString()} />
          <PhaseFact label="Trump" value={formatTrump(round.trump)} />
        </div>
      ) : null}

      {error ? (
        <div className="rounded-lg border border-warning/60 bg-warning/10 px-3 py-2 text-sm text-warning-foreground">
          <p>{error.message}</p>
          {error.traceId ? (
            <p className="text-xs text-warning-foreground/80">
              traceId: {error.traceId}
            </p>
          ) : null}
        </div>
      ) : null}

      <details
        className="rounded-2xl border border-border/60 bg-surface/70"
        open
      >
        <summary className="cursor-pointer list-none rounded-2xl px-4 py-3 text-sm font-semibold text-foreground transition hover:bg-surface">
          Scoreboard
        </summary>
        <div className="px-4 pb-4">
          <ul className="flex flex-col gap-3 text-sm text-muted">
            {scores.map((score, idx) => (
              <li
                key={playerNames[idx]}
                className="flex items-center justify-between rounded-xl border border-border/40 bg-surface/60 px-3 py-2"
              >
                <span className="font-medium text-foreground">
                  {playerNames[idx]}
                </span>
                <span className="text-base font-semibold text-foreground">
                  {score}
                </span>
              </li>
            ))}
          </ul>
        </div>
      </details>
    </aside>
  )
}
