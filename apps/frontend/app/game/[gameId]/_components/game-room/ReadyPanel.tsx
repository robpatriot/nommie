import type { GameRoomViewProps } from '../game-room-view'

interface ReadyPanelProps {
  readyState?: GameRoomViewProps['readyState']
}

export function ReadyPanel({ readyState }: ReadyPanelProps) {
  if (!readyState) {
    return (
      <div className="rounded-2xl border border-dashed border-border bg-surface/70 p-4 text-xs text-subtle">
        Ready controls will appear once interactions are available.
      </div>
    )
  }

  if (!readyState.canReady) {
    return (
      <div className="rounded-2xl border border-border/60 bg-surface/70 p-4 text-sm text-muted">
        <h3 className="mb-1 text-sm font-semibold text-foreground">
          Game in play
        </h3>
        <p>The table is active. Actions will surface here when required.</p>
      </div>
    )
  }

  return (
    <div className="rounded-2xl border border-success/40 bg-success/15 p-4 text-sm text-success-foreground shadow-inner shadow-success/20">
      <h3 className="mb-2 text-sm font-semibold uppercase tracking-[0.4em] text-success-foreground">
        Ready up
      </h3>
      <p className="mb-3 text-xs text-success-foreground/80">
        Mark yourself ready. The game auto-starts when all four seats are ready.
      </p>
      <button
        type="button"
        onClick={() => readyState.onReady()}
        className="w-full rounded-2xl bg-success px-3 py-2 text-sm font-semibold text-success-foreground shadow-lg shadow-success/30 transition hover:bg-success/80 disabled:cursor-not-allowed disabled:bg-success/40 disabled:text-success-foreground/70"
        disabled={readyState.isPending || readyState.hasMarked}
        aria-label={
          readyState.isPending
            ? 'Marking as ready'
            : readyState.hasMarked
              ? 'Ready, waiting for other players'
              : 'Mark yourself as ready'
        }
      >
        {readyState.isPending
          ? 'Marking…'
          : readyState.hasMarked
            ? 'Ready — waiting for others'
            : "I'm ready"}
      </button>
    </div>
  )
}
