'use client'

import { useTranslations } from 'next-intl'
import { useTheme, type ThemeName, THEME_NAMES } from './theme-provider'

// Temporarily disable Oldtime theme option in UI
const DISABLED_THEMES: ThemeName[] = ['oldtime']

const THEME_EMOJI: Record<ThemeName, string> = {
  standard: '🎲',
  high_roller: '🎰',
  oldtime: '🔥',
}

const THEME_OPTIONS = THEME_NAMES.filter(
  (name) => !DISABLED_THEMES.includes(name)
)

export function ThemeSelector({
  preferredTheme,
}: {
  preferredTheme: ThemeName | null
}) {
  const t = useTranslations('settings')

  const {
    themeName,
    setThemeName,
    hydrated,
    isSaving,
    errorMessage,
    clearError,
  } = useTheme()

  const active: ThemeName = hydrated
    ? themeName
    : (preferredTheme ?? 'standard')

  const handleSelect = (nextTheme: ThemeName) => {
    if (hydrated && nextTheme === themeName) return
    clearError()
    void setThemeName(nextTheme)
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-col gap-2">
        {THEME_OPTIONS.map((value) => {
          const isActive = active === value
          const isDisabled = isSaving

          return (
            <button
              key={value}
              type="button"
              onClick={() => handleSelect(value)}
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
                {THEME_EMOJI[value]}
              </span>

              <div className="flex flex-1 flex-col gap-1">
                <span
                  className={`text-sm font-semibold transition ${
                    isActive ? 'text-primary' : 'text-foreground'
                  }`}
                >
                  {t(`theme.options.${value}.label`)}
                </span>
                <span className="text-xs text-muted-foreground">
                  {t(`theme.options.${value}.description`)}
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
