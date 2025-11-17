import type { RoundPublic } from '@/lib/game-room/types'
import { formatTrump } from './utils'
import { AiSeatManager } from './AiSeatManager'
import { ReadyPanel } from './ReadyPanel'
import type { GameRoomViewProps } from '../game-room-view'

interface ScoreSidebarProps {
  playerNames: [string, string, string, string]
  scores: [number, number, number, number]
  round: RoundPublic | null
  readyState?: GameRoomViewProps['readyState']
  aiState?: GameRoomViewProps['aiSeatState']
  isPreGame: boolean
  className?: string
}

export function ScoreSidebar({
  playerNames,
  scores,
  round,
  readyState,
  aiState,
  isPreGame,
  className = '',
}: ScoreSidebarProps) {
  return (
    <aside
      className={`flex h-full flex-col gap-4 rounded-3xl border border-white/10 bg-surface/85 p-5 shadow-[0_25px_80px_rgba(0,0,0,0.35)] backdrop-blur ${className}`}
    >
      <header className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <p className="text-[11px] font-semibold uppercase tracking-[0.4em] text-subtle">
            Scoreboard
          </p>
          <h2 className="text-lg font-semibold text-foreground">
            Tracking the trick race
          </h2>
        </div>
        <span className="rounded-full border border-border/60 px-3 py-1 text-xs text-subtle">
          Updates each sync
        </span>
      </header>

      <details
        className="rounded-2xl border border-border/60 bg-surface/70"
        open
      >
        <summary className="cursor-pointer list-none rounded-2xl px-4 py-3 text-sm font-semibold text-foreground transition hover:bg-surface">
          Cumulative totals
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
          <p>Hand size: {round.hand_size}</p>
          <p>Trump: {formatTrump(round.trump)}</p>
          <p>Tricks won: {round.tricks_won.join(' / ')}</p>
        </div>
      ) : null}

      <ReadyPanel readyState={readyState} />
      {isPreGame ? <AiSeatManager aiState={aiState} /> : null}
    </aside>
  )
}
