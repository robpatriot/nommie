import { useTranslations } from 'next-intl'
import type { GameRoomViewProps } from '../game-room-view'

interface AiSeatManagerProps {
  aiState?: GameRoomViewProps['aiSeatState']
}

export function AiSeatManager({ aiState }: AiSeatManagerProps) {
  const t = useTranslations('game.gameRoom.ai')

  if (!aiState) {
    return (
      <div className="rounded-xl border border-dashed border-border bg-surface/60 p-4 text-sm text-subtle">
        {t('noAccess')}
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
  const activeAiSeats = seats.filter((seat) => seat.isAi)
  const waitingSeatCount = seats.filter((seat) => !seat.isOccupied).length
  const waitingSeatLabel =
    waitingSeatCount === 0
      ? t('waitingSeatLabel.allFilled')
      : t('waitingSeatLabel.openSeats', { count: waitingSeatCount })

  return (
    <div className="rounded-xl border border-border/60 bg-surface/70 p-4 text-sm">
      <header className="mb-4 space-y-2">
        <div className="flex flex-wrap items-center gap-3">
          <p className="text-[11px] font-semibold uppercase tracking-[0.4em] text-subtle">
            {t('kicker')}
          </p>
          <span className="rounded-full border border-border/40 bg-surface/80 px-3 py-1 text-[11px] font-semibold uppercase tracking-wide text-subtle">
            {t('summary', {
              bots: aiState.aiSeats,
              filled: aiState.totalSeats - aiState.availableSeats,
              total: aiState.totalSeats,
            })}
          </span>
        </div>
        <div className="space-y-1">
          <h3 className="text-xl font-semibold text-foreground">
            {t('title')}
          </h3>
          <p className="text-xs text-muted">{t('description')}</p>
        </div>
      </header>

      <div className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center gap-2">
          <button
            type="button"
            onClick={() =>
              aiState.onAdd({ registryName: preferredDefaultName })
            }
            disabled={addDisabled}
            className="relative inline-flex items-center justify-start rounded-md bg-primary pl-3 pr-8 py-2 text-sm font-semibold text-primary-foreground transition hover:bg-primary/80 disabled:cursor-not-allowed disabled:bg-primary/40 disabled:text-primary-foreground/70"
            aria-label={
              aiState.isPending
                ? t('add.aria.adding')
                : t('add.aria.addWithProfile', {
                    profile: preferredDefaultName,
                  })
            }
          >
            <span className="whitespace-nowrap">{t('add.label')}</span>
            {aiState.isPending ? (
              <span className="pointer-events-none absolute inset-y-0 right-2 flex items-center">
                <svg
                  aria-hidden="true"
                  className="h-4 w-4 animate-spin text-primary-foreground"
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
          <span className="text-[11px] text-muted">
            {t('defaultsTo')}&nbsp;
            <span className="font-semibold text-foreground">
              {preferredDefaultName}
            </span>
            {isRegistryLoading ? t('registryLoadingSuffix') : ''}
          </span>
        </div>

        {registryError ? (
          <div className="rounded-md border border-danger/40 bg-danger/10 px-3 py-2 text-xs text-danger-foreground">
            {registryError}
          </div>
        ) : null}

        {activeAiSeats.length > 0 ? (
          <ul className="mt-2 space-y-2 text-xs">
            {activeAiSeats.map((seat, index) => (
              <li
                key={seat.userId ?? `${seat.seat}-${index}`}
                className="rounded-2xl border border-border/60 bg-surface/70 px-4 py-3"
              >
                <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
                  <div className="space-y-1">
                    <p className="text-[11px] font-semibold uppercase tracking-[0.35em] text-subtle">
                      {t('seatLabel', { seatNumber: seat.seat + 1 })}
                    </p>
                    <p className="text-base font-semibold text-foreground">
                      {seat.name}
                    </p>
                    <p className="text-[11px] uppercase tracking-wide text-subtle">
                      {seat.aiProfile
                        ? t('runningProfile', {
                            name: seat.aiProfile.name,
                            version: seat.aiProfile.version,
                          })
                        : t('selectProfileHint')}
                    </p>
                  </div>
                  <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
                    <label htmlFor={`ai-seat-${seat.seat}`} className="sr-only">
                      {t('profileSelect.labelSr', {
                        seatNumber: seat.seat + 1,
                      })}
                    </label>
                    <select
                      id={`ai-seat-${seat.seat}`}
                      aria-label={t('profileSelect.aria', {
                        seatNumber: seat.seat + 1,
                      })}
                      className="rounded-md border border-border bg-background px-2 py-1 text-xs text-foreground focus:border-primary focus:outline-none focus:ring-2 focus:ring-primary/50 disabled:cursor-not-allowed disabled:text-muted"
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
                            ? t('profileSelect.loading')
                            : t('profileSelect.none')}
                        </option>
                      ) : (
                        <>
                          {!seat.aiProfile ? (
                            <option value="" disabled>
                              {t('profileSelect.select')}
                            </option>
                          ) : null}
                          {registryEntries.map((entry) => {
                            const key = `${entry.name}::${entry.version}`
                            return (
                              <option key={key} value={key}>
                                {t('profileOption', {
                                  name: entry.name,
                                  version: entry.version,
                                })}
                              </option>
                            )
                          })}
                        </>
                      )}
                    </select>
                    <button
                      type="button"
                      onClick={() => {
                        aiState.onRemoveSeat?.(seat.seat)
                      }}
                      disabled={aiState.isPending}
                      className="inline-flex h-9 w-9 items-center justify-center rounded-md border border-border/60 text-foreground transition hover:bg-surface-strong disabled:cursor-not-allowed disabled:text-muted"
                      aria-label={t('remove.aria', {
                        seatNumber: seat.seat + 1,
                      })}
                    >
                      <span className="sr-only">
                        {t('remove.sr', { seatNumber: seat.seat + 1 })}
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
                  </div>
                </div>
              </li>
            ))}
          </ul>
        ) : (
          <div className="rounded-2xl border border-dashed border-border bg-surface/70 px-4 py-3 text-xs text-subtle">
            {t('empty')}
          </div>
        )}

        <p className="text-[11px] uppercase tracking-wide text-subtle">
          {waitingSeatLabel}
        </p>
      </div>
    </div>
  )
}
