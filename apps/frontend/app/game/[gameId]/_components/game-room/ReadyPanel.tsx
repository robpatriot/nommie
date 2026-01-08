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
        className={`rounded-2xl border border-dashed border-border bg-card/70 ${
          isCompact ? 'p-3 text-[11px]' : 'p-4 text-xs'
        } text-muted-foreground`}
      >
        {t('ready.none')}
      </div>
    )
  }

  if (!readyState.canReady) {
    return (
      <div
        className={`rounded-2xl border border-border/60 bg-card/70 ${
          isCompact ? 'p-3 text-xs' : 'p-4 text-sm'
        } text-muted-foreground`}
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
      className={`rounded-2xl border border-border/60 bg-card/70 ${
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
          className={`text-muted-foreground ${isCompact ? 'text-[11px]' : 'mb-3 text-xs'}`}
        >
          {t('ready.description')}
        </p>
      </div>
      <button
        type="button"
        onClick={() => {
          readyState.onReady()
        }}
        className={`rounded-2xl text-sm font-semibold shadow-lg transition disabled:cursor-not-allowed disabled:opacity-50 ${
          readyState.hasMarked
            ? 'bg-muted text-muted-foreground shadow-muted/30 hover:bg-muted/80'
            : 'bg-primary text-primary-foreground shadow-primary/30 hover:bg-primary/80'
        } ${isCompact ? 'w-full px-4 py-2 sm:w-auto' : 'w-full px-3 py-2'}`}
        disabled={readyState.isPending}
        aria-label={
          readyState.isPending
            ? readyState.hasMarked
              ? t('ready.button.aria.unmarking')
              : t('ready.button.aria.marking')
            : readyState.hasMarked
              ? t('ready.button.aria.markNotReady')
              : t('ready.button.aria.markReady')
        }
      >
        {readyState.isPending
          ? readyState.hasMarked
            ? t('ready.button.unmarking')
            : t('ready.button.marking')
          : readyState.hasMarked
            ? t('ready.button.markNotReady')
            : t('ready.button.label')}
      </button>
    </div>
  )
}
