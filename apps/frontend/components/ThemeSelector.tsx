'use client'

import { useEffect, useState, useTransition } from 'react'
import { useTranslations } from 'next-intl'
import { updateThemeAction } from '@/app/actions/settings-actions'
import { useTheme, type ThemeName } from './theme-provider'

// Temporarily disable Oldtime theme option in UI
const DISABLED_THEMES: ThemeName[] = ['oldtime']

const ALL_THEME_OPTIONS = [
  {
    value: 'standard',
    emoji: '🎲',
  },
  {
    value: 'high_roller',
    emoji: '🎰',
  },
  {
    value: 'oldtime',
    emoji: '🔥',
  },
] satisfies Array<{ value: ThemeName; emoji: string }>

const THEME_OPTIONS = ALL_THEME_OPTIONS.filter(
  (option) => !DISABLED_THEMES.includes(option.value)
)

export function ThemeSelector({
  preferredTheme,
}: {
  preferredTheme: ThemeName | null
}) {
  const t = useTranslations('settings')
  const { themeName, setThemeName, hydrated } = useTheme()
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [isPending, startTransition] = useTransition()

  // Sync backend preference to localStorage on mount
  useEffect(() => {
    if (typeof window === 'undefined') {
      return
    }

    try {
      const backendPreference: ThemeName = preferredTheme ?? 'standard'
      const stored = window.localStorage.getItem('nommie.theme_name')

      // Only sync if backend preference differs from localStorage
      if (stored !== backendPreference) {
        window.localStorage.setItem('nommie.theme_name', backendPreference)
        if (hydrated) {
          setThemeName(backendPreference)
        }
      }
    } catch {
      // Ignore storage access errors (e.g., in private browsing)
    }
  }, [preferredTheme, hydrated, setThemeName])

  const active = hydrated ? themeName : (preferredTheme ?? 'standard')

  const handleSelect = (theme: ThemeName) => {
    if (hydrated && theme === themeName) {
      return
    }

    const previousTheme = themeName
    setErrorMessage(null)
    setThemeName(theme)

    startTransition(async () => {
      const result = await updateThemeAction(theme)
      if (result.kind === 'error') {
        setErrorMessage(result.message)
        setThemeName(previousTheme)
      } else {
        setErrorMessage(null)
      }
    })
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-col gap-2">
        {THEME_OPTIONS.map((option) => {
          const isActive = active === option.value
          const isDisabled = isPending

          return (
            <button
              key={option.value}
              type="button"
              onClick={() => handleSelect(option.value)}
              disabled={isDisabled}
              className={`
                group relative flex items-center gap-4 rounded-2xl border px-5 py-4 text-left transition
                ${
                  isActive
                    ? 'border-primary bg-primary/10'
                    : 'border-border/60 bg-card/50 hover:border-primary/40 hover:bg-card/80'
                }
                ${isDisabled ? 'cursor-not-allowed opacity-60' : 'cursor-pointer'}
                focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary
              `}
            >
              <span className="text-3xl" aria-hidden="true">
                {option.emoji}
              </span>
              <div className="flex flex-1 flex-col gap-1">
                <span
                  className={`text-sm font-semibold transition ${
                    isActive ? 'text-primary' : 'text-foreground'
                  }`}
                >
                  {t(`theme.options.${option.value}.label`)}
                </span>
                <span className="text-xs text-muted-foreground">
                  {t(`theme.options.${option.value}.description`)}
                </span>
              </div>
              {isActive && (
                <div
                  className="flex size-5 items-center justify-center rounded-full bg-primary text-xs text-primary-foreground"
                  aria-label={t('theme.selected')}
                >
                  ✓
                </div>
              )}
            </button>
          )
        })}
      </div>
      {errorMessage && (
        <div className="min-h-[1.5rem] text-sm text-destructive">
          {errorMessage}
        </div>
      )}
    </div>
  )
}
