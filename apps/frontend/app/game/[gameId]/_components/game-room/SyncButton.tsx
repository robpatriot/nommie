import { useTranslations } from 'next-intl'
import { cn } from '@/lib/cn'

interface SyncButtonProps {
  onRefresh: () => void
  isRefreshing: boolean
  className?: string
}

/**
 * Shared sync button component for refreshing game state.
 * Used in both desktop (game-room-view) and mobile (TrickArea) layouts.
 */
export function SyncButton({
  onRefresh,
  isRefreshing,
  className,
}: SyncButtonProps) {
  const t = useTranslations('game.gameRoom.actions')

  return (
    <button
      type="button"
      onClick={onRefresh}
      disabled={isRefreshing}
      className={cn(
        'flex items-center gap-2 rounded-full border border-white/20 bg-black/40 px-3 py-1.5 text-[11px] font-semibold text-white transition hover:border-primary/60 hover:bg-primary/20 disabled:cursor-not-allowed disabled:opacity-60',
        className
      )}
      aria-label={isRefreshing ? t('refreshingAria') : t('refreshAria')}
    >
      <span>{t('sync')}</span>
      <svg
        aria-hidden="true"
        className={`h-4 w-4 ${isRefreshing ? 'animate-spin' : ''}`}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth={1.8}
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M21 2v6h-6" />
        <path d="M3 22v-6h6" />
        <path d="M3.51 9a9 9 0 0 1 14.85-3.36L21 8" />
        <path d="M20.49 15a9 9 0 0 1-14.85 3.36L3 16" />
      </svg>
    </button>
  )
}
