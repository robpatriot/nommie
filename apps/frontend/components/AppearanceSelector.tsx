'use client'

import { useMemo, useState, useTransition } from 'react'
import { useTranslations } from 'next-intl'
import { updateAppearanceAction } from '@/app/actions/settings-actions'
import { useTheme, type ThemeMode } from './theme-provider'

const OPTIONS: Array<{
  value: ThemeMode
  emoji: string
}> = [
  {
    value: 'system',
    emoji: '🖥️',
  },
  {
    value: 'light',
    emoji: '🌞',
  },
  {
    value: 'dark',
    emoji: '🌙',
  },
]

export function AppearanceSelector() {
  const t = useTranslations('settings')
  const { theme, setTheme, hydrated } = useTheme()
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [isPending, startTransition] = useTransition()

  const active = useMemo<ThemeMode>(() => {
    if (hydrated) {
      return theme
    }
    return 'system'
  }, [hydrated, theme])

  const handleSelect = (mode: ThemeMode) => {
    if (hydrated && mode === theme) {
      return
    }

    const previousTheme = theme
    setErrorMessage(null)
    setTheme(mode)

    startTransition(async () => {
      const result = await updateAppearanceAction(mode)
      if (result.kind === 'error') {
        setErrorMessage(result.message)
        setTheme(previousTheme)
      } else {
        setErrorMessage(null)
      }
    })
  }

  return (
    <div
      className="flex flex-col gap-4"
      role="radiogroup"
      aria-label={t('appearance.ariaLabel')}
      aria-busy={isPending}
    >
      <div className="flex flex-col gap-3">
        {OPTIONS.map((option) => {
          const isActive = active === option.value
          return (
            <button
              key={option.value}
              type="button"
              onClick={() => handleSelect(option.value)}
              aria-pressed={isActive}
              disabled={isPending}
              className={`flex items-center gap-4 rounded-2xl border px-4 py-3 text-left transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary ${
                isActive
                  ? 'border-primary/60 bg-primary/10 text-foreground shadow-inner shadow-primary/20'
                  : 'border-border/60 bg-surface/80 text-muted hover:border-primary/40 hover:text-foreground'
              } ${isPending ? 'opacity-80' : ''}`}
            >
              <span aria-hidden className="text-xl">
                {option.emoji}
              </span>
              <span className="flex flex-1 flex-col">
                <span className="text-sm font-semibold text-foreground">
                  {t(`appearance.options.${option.value}.label`)}
                </span>
                <span className="text-xs text-subtle">
                  {t(`appearance.options.${option.value}.description`)}
                </span>
              </span>
              {isActive ? (
                <span
                  aria-hidden
                  className="inline-flex h-6 w-6 items-center justify-center rounded-full bg-primary text-xs font-semibold text-primary-foreground"
                >
                  ✓
                </span>
              ) : (
                <span
                  aria-hidden
                  className="inline-flex h-6 w-6 items-center justify-center rounded-full border border-border/60 text-xs text-muted"
                >
                  ○
                </span>
              )}
            </button>
          )
        })}
      </div>
      <div className="min-h-[1.5rem] text-sm">
        {isPending ? (
          <span className="text-muted">{t('appearance.status.saving')}</span>
        ) : errorMessage ? (
          <span className="text-danger">
            {t('appearance.status.couldNotSave', { error: errorMessage })}
          </span>
        ) : (
          <span className="text-subtle">{t('appearance.status.saved')}</span>
        )}
      </div>
    </div>
  )
}
