import type { GameRoomViewProps } from '../game-room-view'

interface ReadyPanelProps {
  readyState?: GameRoomViewProps['readyState']
}

export function ReadyPanel({ readyState }: ReadyPanelProps) {
  if (!readyState) {
    return (
      <div className="rounded-xl border border-dashed border-border bg-surface/60 p-4 text-xs text-subtle">
        Ready controls will appear once interactions are available.
      </div>
    )
  }

  if (!readyState.canReady) {
    return (
      <div className="rounded-xl border border-border bg-surface/60 p-4 text-sm text-muted">
        <h3 className="mb-1 text-sm font-semibold text-foreground">
          Game in play
        </h3>
        <p>The table is active. Actions will surface here when required.</p>
      </div>
    )
  }

  return (
    <div className="rounded-xl border border-success/40 bg-success/10 p-4 text-sm text-success-foreground">
      <h3 className="mb-2 text-sm font-semibold text-success-foreground">
        Ready Up
      </h3>
      <p className="mb-3 text-xs text-success-foreground/80">
        Mark yourself ready. The game auto-starts when all four seats are ready.
      </p>
      <button
        type="button"
        onClick={() => readyState.onReady()}
        className="w-full rounded-md bg-success px-3 py-2 text-sm font-semibold text-success-foreground transition hover:bg-success/80 disabled:cursor-not-allowed disabled:bg-success/40 disabled:text-success-foreground/70"
        disabled={readyState.isPending || readyState.hasMarked}
      >
        {readyState.isPending
          ? 'Marking…'
          : readyState.hasMarked
            ? 'Ready — waiting for others'
            : "I'm Ready"}
      </button>
    </div>
  )
}
