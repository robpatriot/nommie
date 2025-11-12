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
}

export function ScoreSidebar({
  playerNames,
  scores,
  round,
  readyState,
  aiState,
  isPreGame,
}: ScoreSidebarProps) {
  return (
    <aside className="flex h-full flex-col gap-4 rounded-2xl border border-border bg-surface/70 p-4">
      <header className="flex items-center justify-between">
        <h2 className="text-base font-semibold text-foreground">Scores</h2>
        <span className="text-xs text-subtle">Updated each sync</span>
      </header>

      <details className="rounded-xl border border-border bg-surface/60" open>
        <summary className="cursor-pointer list-none rounded-xl px-4 py-3 text-sm font-medium text-foreground transition hover:bg-surface">
          Cumulative Totals
        </summary>
        <div className="px-4 pb-3">
          <ul className="flex flex-col gap-2 text-sm text-muted">
            {scores.map((score, idx) => (
              <li
                key={playerNames[idx]}
                className="flex items-center justify-between"
              >
                <span>{playerNames[idx]}</span>
                <span className="font-semibold text-foreground">{score}</span>
              </li>
            ))}
          </ul>
        </div>
      </details>

      {round ? (
        <div className="rounded-xl border border-border bg-surface/60 p-4 text-sm text-muted">
          <div className="mb-2 text-xs uppercase tracking-wide text-subtle">
            Round Snapshot
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
