import type { GameRoomViewProps } from '../game-room-view'

interface AiSeatManagerProps {
  aiState?: GameRoomViewProps['aiSeatState']
}

export function AiSeatManager({ aiState }: AiSeatManagerProps) {
  if (!aiState) {
    return (
      <div className="rounded-xl border border-dashed border-border bg-surface/60 p-4 text-sm text-subtle">
        The host manages AI seats for this table. You&apos;ll see each seat
        update in real time as they adjust the lineup.
      </div>
    )
  }

  const { seats } = aiState
  const registry = aiState.registry
  const registryEntries = registry?.entries ?? []
  const isRegistryLoading = registry?.isLoading ?? false
  const registryError = registry?.error ?? null
  const preferredDefaultName =
    registry?.defaultName ??
    registryEntries.find((entry) => entry.name === 'HeuristicV1')?.name ??
    registryEntries[0]?.name ??
    'HeuristicV1'
  const addDisabled =
    !aiState.canAdd ||
    aiState.isPending ||
    (aiState.registry?.isLoading ?? false)

  return (
    <div className="rounded-xl border border-accent/40 bg-accent/10 p-4 text-sm text-accent-contrast">
      <header className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <div>
          <h3 className="text-sm font-semibold text-accent-contrast">
            AI Seats
          </h3>
          <p className="text-xs text-accent-contrast/80">
            Use bots to fill empty seats before the game starts.
          </p>
        </div>
        <span className="rounded-full border border-accent/40 bg-accent/15 px-3 py-1 text-[11px] font-semibold uppercase tracking-wide text-accent-contrast">
          {aiState.aiSeats} bots · {aiState.totalSeats - aiState.availableSeats}
          /{aiState.totalSeats} seats filled
        </span>
      </header>

      <div className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center gap-2">
          <button
            type="button"
            onClick={() =>
              aiState.onAdd({ registryName: preferredDefaultName })
            }
            disabled={addDisabled}
            className="relative inline-flex items-center justify-start rounded-md bg-accent pl-3 pr-8 py-2 text-sm font-semibold text-accent-foreground transition hover:bg-accent/80 disabled:cursor-not-allowed disabled:bg-accent/40 disabled:text-accent-foreground/70"
            aria-label={
              aiState.isPending
                ? 'Adding AI player'
                : `Add AI player with profile ${preferredDefaultName}`
            }
          >
            <span className="whitespace-nowrap">Add AI</span>
            {aiState.isPending ? (
              <span className="pointer-events-none absolute inset-y-0 right-2 flex items-center">
                <svg
                  aria-hidden="true"
                  className="h-4 w-4 animate-spin text-accent-foreground"
                  viewBox="0 0 24 24"
                  fill="none"
                >
                  <circle
                    className="opacity-25"
                    cx="12"
                    cy="12"
                    r="10"
                    stroke="currentColor"
                    strokeWidth="4"
                  />
                  <path
                    className="opacity-75"
                    d="M4 12a8 8 0 0 1 8-8"
                    stroke="currentColor"
                    strokeWidth="4"
                    strokeLinecap="round"
                  />
                </svg>
              </span>
            ) : null}
          </button>
          <span className="text-[11px] text-accent-contrast/80">
            Defaults to&nbsp;
            <span className="font-semibold text-accent-contrast">
              {preferredDefaultName}
            </span>
            {isRegistryLoading ? ' (loading registry…)' : ''}
          </span>
        </div>

        {registryError ? (
          <div className="rounded-md border border-danger/40 bg-danger/10 px-3 py-2 text-xs text-danger-foreground">
            {registryError}
          </div>
        ) : null}

        <ul className="mt-2 space-y-2 text-xs">
          {seats.map((seat, index) => (
            <li
              key={seat.userId ?? `${seat.seat}-${index}`}
              className="rounded-lg border border-accent/30 bg-surface/60 px-3 py-3"
            >
              <div className="flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between">
                <div className="flex flex-col">
                  <span className="font-semibold text-foreground">
                    Seat {seat.seat + 1}
                  </span>
                  <span className="text-[11px] uppercase tracking-wide text-subtle">
                    {seat.isOccupied
                      ? [
                          seat.isAi ? 'AI-controlled' : 'Human player',
                          seat.isReady ? 'Ready' : 'Not ready',
                        ].join(' • ')
                      : 'Open seat'}
                  </span>
                  {seat.isAi && seat.aiProfile ? (
                    <span className="text-[11px] text-accent-contrast/90">
                      Profile:{' '}
                      <span className="font-medium text-accent-contrast">
                        {seat.aiProfile.name}
                      </span>{' '}
                      · v{seat.aiProfile.version}
                    </span>
                  ) : null}
                </div>
                <div className="mt-2 flex items-center gap-2 sm:mt-0">
                  {seat.isAi ? (
                    <>
                      <label
                        htmlFor={`ai-seat-${seat.seat}`}
                        className="sr-only"
                      >
                        Select AI profile for seat {seat.seat + 1}
                      </label>
                      <select
                        id={`ai-seat-${seat.seat}`}
                        aria-label={`Select AI profile for seat ${seat.seat + 1}`}
                        className="rounded-md border border-border bg-background px-2 py-1 text-xs text-foreground focus:border-accent focus:outline-none focus:ring-2 focus:ring-accent/50 disabled:cursor-not-allowed disabled:text-muted"
                        disabled={
                          aiState.isPending ||
                          isRegistryLoading ||
                          registryEntries.length === 0 ||
                          !aiState.onUpdateSeat
                        }
                        value={
                          seat.aiProfile
                            ? `${seat.aiProfile.name}::${seat.aiProfile.version}`
                            : ''
                        }
                        onChange={(event) => {
                          const value = event.target.value
                          if (!value || !aiState.onUpdateSeat) {
                            return
                          }
                          const [registryName, registryVersion] =
                            value.split('::')
                          aiState.onUpdateSeat(seat.seat, {
                            registryName,
                            registryVersion,
                          })
                        }}
                      >
                        {registryEntries.length === 0 ? (
                          <option value="">
                            {isRegistryLoading
                              ? 'Loading profiles…'
                              : 'No profiles available'}
                          </option>
                        ) : (
                          registryEntries.map((entry) => {
                            const key = `${entry.name}::${entry.version}`
                            return (
                              <option key={key} value={key}>
                                {entry.name} · v{entry.version}
                              </option>
                            )
                          })
                        )}
                      </select>
                      <button
                        type="button"
                        onClick={() => {
                          aiState.onRemoveSeat?.(seat.seat)
                        }}
                        disabled={aiState.isPending}
                        className="inline-flex h-8 w-8 items-center justify-center rounded-md border border-accent/40 text-accent-foreground transition hover:bg-accent/20 disabled:cursor-not-allowed disabled:text-accent-foreground/60"
                        aria-label={`Remove AI from seat ${seat.seat + 1}`}
                      >
                        <span className="sr-only">
                          Remove AI from seat {seat.seat + 1}
                        </span>
                        <svg
                          aria-hidden="true"
                          className="h-4 w-4"
                          viewBox="0 0 24 24"
                          fill="none"
                          stroke="currentColor"
                          strokeWidth={1.5}
                          strokeLinecap="round"
                          strokeLinejoin="round"
                        >
                          <path d="M4 7h16" />
                          <path d="M9 7V4h6v3" />
                          <path d="M10 11v6" />
                          <path d="M14 11v6" />
                          <path d="M6 7v12a1 1 0 0 0 1 1h10a1 1 0 0 0 1-1V7" />
                        </svg>
                      </button>
                    </>
                  ) : (
                    <span className="rounded-md border border-accent/40 bg-accent/15 px-2 py-1 text-[11px] text-accent-contrast/80">
                      {seat.isOccupied ? 'Human player' : 'Awaiting player'}
                    </span>
                  )}
                </div>
              </div>
            </li>
          ))}
        </ul>
      </div>
    </div>
  )
}
