import type { PhaseSnapshot, RoundPublic, Seat } from '@/lib/game-room/types'
import { formatTrump, getPhaseLabel } from './utils'
import { PhaseFact } from './PhaseFact'

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
  bidStatus?: Array<{
    seat: number
    name: string
    bid: number | null
    isActive: boolean
  }>
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
  bidStatus = [],
  onRefresh,
  isRefreshing = false,
  className = '',
}: ScoreSidebarProps) {
  return (
    <aside
      className={`flex h-full flex-col gap-4 rounded-3xl border border-white/10 bg-surface/85 p-5 shadow-[0_25px_80px_rgba(0,0,0,0.35)] backdrop-blur ${className}`}
    >
      <header className="flex flex-col gap-3">
        <div>
          <p className="text-[11px] font-semibold uppercase tracking-[0.4em] text-subtle">
            Game #{gameId}
          </p>
          <h2 className="text-xl font-semibold text-foreground">
            {getPhaseLabel(phase)}
          </h2>
          <p className="text-xs font-medium uppercase tracking-[0.35em] text-subtle">
            Turn: <span className="text-sm text-foreground">{activeName}</span>
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          {phase.phase === 'Trick' ? (
            <span className="rounded-full bg-surface px-3 py-1 text-xs font-semibold uppercase tracking-[0.35em] text-subtle">
              Trick {phase.data.trick_no} / {round?.hand_size ?? '?'}
            </span>
          ) : null}
          {onRefresh ? (
            <button
              type="button"
              onClick={onRefresh}
              disabled={isRefreshing}
              className="rounded-full border border-white/20 px-3 py-1 text-xs font-semibold uppercase tracking-[0.35em] text-foreground transition hover:border-primary/60 hover:text-primary disabled:opacity-60"
              aria-label="Refresh game state"
            >
              {isRefreshing ? 'Syncing…' : 'Refresh'}
            </button>
          ) : null}
        </div>
      </header>

      {round ? (
        <div className="grid gap-2 rounded-2xl border border-border/60 bg-surface/70 p-3 text-sm text-muted">
          <PhaseFact label="Round" value={`#${roundNo}`} />
          <PhaseFact label="Hand Size" value={round.hand_size.toString()} />
          <PhaseFact label="Dealer" value={seatDisplayName(dealer)} />
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

      {bidStatus.length > 0 ? (
        <div className="rounded-2xl border border-border/60 bg-surface/70 p-3">
          <p className="mb-2 text-xs font-semibold uppercase tracking-[0.35em] text-subtle">
            Bidding Status
          </p>
          <div className="flex flex-wrap gap-2">
            {bidStatus.map(({ seat, name, bid, isActive }) => (
              <div
                key={seat}
                className={`flex items-center gap-2 rounded-xl border px-3 py-1.5 text-xs font-semibold uppercase tracking-[0.3em] ${
                  isActive
                    ? 'border-success bg-success/15 text-success-contrast'
                    : 'border-white/15 bg-surface text-muted'
                }`}
              >
                <span className="text-[10px] tracking-[0.2em] text-subtle">
                  {name}
                </span>
                <span className="text-sm font-semibold text-foreground">
                  {bid ?? '—'}
                </span>
              </div>
            ))}
          </div>
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

      {round ? (
        <div className="rounded-2xl border border-border/60 bg-surface/70 p-4 text-sm text-muted">
          <div className="mb-2 text-xs uppercase tracking-wide text-subtle">
            Round snapshot
          </div>
          <p>Tricks won: {round.tricks_won.join(' / ')}</p>
        </div>
      ) : null}
    </aside>
  )
}
