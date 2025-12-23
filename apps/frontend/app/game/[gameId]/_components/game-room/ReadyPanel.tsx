import { useTranslations } from 'next-intl'
import type { GameRoomViewProps } from '../game-room-view'

interface ReadyPanelProps {
  readyState?: GameRoomViewProps['readyState']
  variant?: 'default' | 'compact'
}

export function ReadyPanel({
  readyState,
  variant = 'default',
}: ReadyPanelProps) {
  const t = useTranslations('game.gameRoom')
  const isCompact = variant === 'compact'

  if (!readyState) {
    return (
      <div
        className={`rounded-2xl border border-dashed border-border bg-surface/70 ${
          isCompact ? 'p-3 text-[11px]' : 'p-4 text-xs'
        } text-subtle`}
      >
        {t('ready.none')}
      </div>
    )
  }

  if (!readyState.canReady) {
    return (
      <div
        className={`rounded-2xl border border-border/60 bg-surface/70 ${
          isCompact ? 'p-3 text-xs' : 'p-4 text-sm'
        } text-muted`}
      >
        <h3
          className={`mb-1 font-semibold text-foreground ${
            isCompact ? 'text-xs' : 'text-sm'
          }`}
        >
          {t('ready.inPlayTitle')}
        </h3>
        <p>{t('ready.inPlayDescription')}</p>
      </div>
    )
  }

  return (
    <div
      className={`rounded-2xl border border-border/60 bg-surface/70 ${
        isCompact
          ? 'flex flex-col gap-3 p-4 text-xs sm:flex-row sm:items-center sm:justify-between'
          : 'p-4 text-sm'
      }`}
    >
      <div>
        <h3
          className={`mb-1 font-semibold text-foreground ${
            isCompact ? 'text-xs' : 'mb-2 text-sm'
          }`}
        >
          {t('ready.title')}
        </h3>
        <p
          className={`text-muted ${isCompact ? 'text-[11px]' : 'mb-3 text-xs'}`}
        >
          {t('ready.description')}
        </p>
      </div>
      <button
        type="button"
        onClick={() => {
          readyState.onReady()
        }}
        className={`rounded-2xl bg-primary text-sm font-semibold text-primary-foreground shadow-lg shadow-primary/30 transition hover:bg-primary/80 disabled:cursor-not-allowed disabled:bg-primary/40 disabled:text-primary-foreground/70 ${
          isCompact ? 'w-full px-4 py-2 sm:w-auto' : 'w-full px-3 py-2'
        }`}
        disabled={readyState.isPending || readyState.hasMarked}
        aria-label={
          readyState.isPending
            ? t('ready.button.aria.marking')
            : readyState.hasMarked
              ? t('ready.button.aria.readyWaiting')
              : t('ready.button.aria.markReady')
        }
      >
        {readyState.isPending
          ? t('ready.button.marking')
          : readyState.hasMarked
            ? t('ready.button.readyWaiting')
            : t('ready.button.label')}
      </button>
    </div>
  )
}
