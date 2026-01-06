import { useLocale, useTranslations } from 'next-intl'
import type { PhaseSnapshot, RoundPublic, Seat } from '@/lib/game-room/types'
import { PhaseFact } from './PhaseFact'
import { StatCard } from '@/components/StatCard'
import { cn } from '@/lib/cn'
import { formatNumber } from '@/utils/number-formatting'
import { getPhaseTranslationKey, isTrickPhase } from './phase-helpers'
import { formatVersion } from '@/utils/version-formatting'

interface AiProfile {
  name: string
  version: string
}

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
  aiProfiles?: [
    AiProfile | null,
    AiProfile | null,
    AiProfile | null,
    AiProfile | null,
  ]
  error?: { message: string; traceId?: string } | null
  onShowHistory?: () => void
  isHistoryLoading?: boolean
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
  aiProfiles,
  error,
  onShowHistory,
  isHistoryLoading = false,
  className,
}: ScoreSidebarProps) {
  const locale = useLocale()
  const t = useTranslations('game.gameRoom.sidebar')
  const tPhase = useTranslations('game.gameRoom.phase')
  const tTrump = useTranslations('game.gameRoom.trump')
  const tError = useTranslations('game.gameRoom.error')

  return (
    <aside
      className={cn(
        'flex h-full flex-col gap-4 rounded-3xl border border-white/10 bg-surface/85 p-5 shadow-elevated backdrop-blur',
        className
      )}
    >
      <header className="space-y-3 rounded-2xl p-4">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="min-w-0">
            <p className="text-[10px] font-semibold uppercase tracking-[0.4em] text-subtle">
              {t('kicker', { gameId })}
            </p>
            <h2 className="text-2xl font-bold text-foreground">
              {tPhase(getPhaseTranslationKey(phase.phase))}
            </h2>
            <p className="text-sm text-muted">
              {t('turnLabel')}{' '}
              <span className="font-semibold text-primary">{activeName}</span>
            </p>
          </div>
          {isTrickPhase(phase) ? (
            <StatCard
              label={t('trick.label')}
              value={`${phase.data.trick_no} / ${round?.hand_size ?? '?'}`}
              className="px-3 py-1.5"
              valueClassName="text-base"
            />
          ) : null}
        </div>
      </header>

      {round ? (
        <div className="grid grid-cols-2 gap-x-4 gap-y-3 rounded-2xl border border-border/60 bg-surface/70 p-3 text-sm text-muted">
          <PhaseFact
            label={t('facts.round')}
            value={formatNumber(roundNo, locale)}
          />
          <PhaseFact
            label={t('facts.dealer')}
            value={seatDisplayName(dealer)}
          />
          <PhaseFact
            label={t('facts.handSize')}
            value={formatNumber(round.hand_size, locale)}
          />
          <PhaseFact
            label={t('facts.trump')}
            value={round.trump ? tTrump(round.trump) : tTrump('undeclared')}
          />
        </div>
      ) : null}

      {error ? (
        <div className="rounded-lg border border-warning/60 bg-warning/10 px-3 py-2 text-sm text-warning-foreground">
          <p>{error.message}</p>
          {error.traceId ? (
            <p className="text-xs text-warning-foreground/80">
              {tError('traceIdLabel')}: {error.traceId}
            </p>
          ) : null}
        </div>
      ) : null}

      <details
        className="rounded-2xl border border-border/60 bg-surface/70"
        open
      >
        <summary className="flex cursor-pointer flex-wrap items-center justify-between gap-3 rounded-2xl px-4 py-3 text-sm font-semibold text-foreground transition hover:bg-surface">
          <span>{t('scoreboard.title')}</span>
          {onShowHistory ? (
            <button
              type="button"
              onClick={(event) => {
                event.preventDefault()
                onShowHistory()
              }}
              disabled={isHistoryLoading}
              className="flex items-center gap-1 rounded-full border border-white/20 bg-surface/60 px-3 py-1 text-[11px] font-semibold text-foreground transition hover:border-primary/60 hover:bg-primary/10 hover:text-primary disabled:cursor-not-allowed disabled:opacity-60"
              aria-label={t('scoreboard.showHistoryAria')}
            >
              <span>
                {isHistoryLoading
                  ? t('scoreboard.opening')
                  : t('scoreboard.history')}
              </span>
              <svg
                aria-hidden="true"
                className="h-3 w-3"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth={1.8}
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="M6 2h9l5 5v15H6z" />
                <path d="M14 2v6h6" />
                <path d="M8 13h8" />
                <path d="M8 17h5" />
              </svg>
            </button>
          ) : null}
        </summary>
        <div className="px-4 pb-4">
          <ul className="flex flex-col gap-3 text-sm text-muted">
            {scores.map((score, idx) => {
              const aiProfile = aiProfiles?.[idx]
              return (
                <li
                  key={playerNames[idx]}
                  className="flex items-center justify-between rounded-xl border border-border/40 bg-surface/60 px-3 py-2"
                >
                  <div className="flex min-w-0 flex-1 items-center gap-2">
                    <span className="font-medium text-foreground">
                      {playerNames[idx]}
                    </span>
                    {aiProfile ? (
                      <span
                        className="inline-flex items-center rounded-full bg-black/20 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-foreground"
                        title={`AI: ${aiProfile.name} v${formatVersion(aiProfile.version)}`}
                      >
                        {aiProfile.name}
                      </span>
                    ) : null}
                  </div>
                  <span className="text-base font-semibold text-foreground">
                    {score}
                  </span>
                </li>
              )
            })}
          </ul>
        </div>
      </details>
    </aside>
  )
}
