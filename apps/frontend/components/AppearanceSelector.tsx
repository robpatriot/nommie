'use client'

import { useEffect, useMemo, useState, useTransition } from 'react'
import { useTranslations } from 'next-intl'
import { updateAppearanceAction } from '@/app/actions/settings-actions'
import { useTheme, type ColourScheme } from './theme-provider'

const STORAGE_KEY = 'nommie.colour_scheme'

const SPECIFIC_OPTIONS: Array<{
  value: ColourScheme
  emoji: string
}> = [
  {
    value: 'light',
    emoji: '🌞',
  },
  {
    value: 'dark',
    emoji: '🌙',
  },
]

export function AppearanceSelector({
  preferredAppearance,
}: {
  preferredAppearance: ColourScheme | null
}) {
  const t = useTranslations('settings')
  const { colourScheme, setColourScheme, resolvedColourScheme, hydrated } =
    useTheme()
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [isPending, startTransition] = useTransition()

  // Sync backend preference to localStorage on mount
  // This ensures cross-device sync: if user changes preference on one device,
  // it will sync to localStorage on other devices when they visit settings
  useEffect(() => {
    if (typeof window === 'undefined') {
      return
    }

    try {
      const backendPreference: ColourScheme = preferredAppearance ?? 'system'
      const stored = window.localStorage.getItem(STORAGE_KEY)

      // Only sync if backend preference differs from localStorage
      if (backendPreference === 'system') {
        // Backend says system, but localStorage might have a value
        if (stored !== null && stored !== 'system') {
          window.localStorage.removeItem(STORAGE_KEY)
        }
      } else {
        // Backend has explicit preference
        if (stored !== backendPreference) {
          window.localStorage.setItem(STORAGE_KEY, backendPreference)
        }
      }
    } catch {
      // Ignore storage access errors (e.g., in private browsing)
    }
  }, [preferredAppearance])

  // null means no explicit preference (use system default)
  // 'system' means explicitly set to system
  // 'light'/'dark' means explicitly set to that mode
  const isUsingPreference =
    preferredAppearance !== null && preferredAppearance !== 'system'

  const active = useMemo<ColourScheme>(() => {
    if (hydrated) {
      return colourScheme
    }
    return preferredAppearance ?? 'system'
  }, [hydrated, colourScheme, preferredAppearance])

  const effectiveLabel =
    resolvedColourScheme === 'dark'
      ? t('appearance.options.dark.label')
      : t('appearance.options.light.label')

  const handleSelect = (mode: ColourScheme) => {
    if (hydrated && mode === colourScheme) {
      return
    }

    const previousTheme = colourScheme
    setErrorMessage(null)
    setColourScheme(mode)

    startTransition(async () => {
      const result = await updateAppearanceAction(mode)
      if (result.kind === 'error') {
        setErrorMessage(result.message)
        setColourScheme(previousTheme)
      } else {
        setErrorMessage(null)
      }
    })
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-col gap-2">
        {/* Specific appearance options */}
        {SPECIFIC_OPTIONS.map((option) => {
          const isActive = active === option.value
          return (
            <button
              key={option.value}
              type="button"
              onClick={() => handleSelect(option.value)}
              disabled={isPending}
              aria-pressed={isActive}
              className={`flex items-center justify-between rounded-2xl border px-4 py-3 text-left transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary ${
                isActive
                  ? 'border-primary/60 bg-primary/10 text-foreground shadow-inner shadow-primary/20'
                  : 'border-border/60 bg-card/80 text-muted-foreground hover:border-primary/40 hover:text-foreground'
              } ${isPending ? 'opacity-80' : ''}`}
            >
              <span className="flex items-center gap-3">
                <span aria-hidden className="text-xl">
                  {option.emoji}
                </span>
                <span className="flex flex-col">
                  <span className="text-sm font-semibold text-foreground">
                    {t(`appearance.options.${option.value}.label`)}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    {t(`appearance.options.${option.value}.description`)}
                  </span>
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
                  className="inline-flex h-6 w-6 items-center justify-center rounded-full border border-border/60 text-xs text-muted-foreground"
                >
                  ○
                </span>
              )}
            </button>
          )
        })}

        {/* Visual separator */}
        <div className="my-2 flex items-center gap-3">
          <div className="h-px flex-1 bg-border/30" />
          <span className="text-xs uppercase tracking-wide text-muted-foreground">
            {t('appearance.separator')}
          </span>
          <div className="h-px flex-1 bg-border/30" />
        </div>

        {/* System default option */}
        <button
          type="button"
          onClick={() => handleSelect('system')}
          disabled={isPending}
          aria-pressed={active === 'system'}
          className={`flex items-center justify-between rounded-2xl border px-4 py-3 text-left transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary ${
            active === 'system'
              ? 'border-primary/60 bg-primary/10 text-foreground shadow-inner shadow-primary/20'
              : 'border-dashed border-muted/40 bg-card/40 text-muted-foreground hover:border-primary/40 hover:bg-card/60 hover:text-foreground'
          } ${isPending ? 'opacity-80' : ''}`}
        >
          <span className="flex items-center gap-3">
            <span aria-hidden className="text-xl">
              🖥️
            </span>
            <span className="flex flex-col">
              <span className="text-sm font-semibold text-foreground">
                {t('appearance.options.system.label')}
              </span>
              <span className="text-xs text-muted-foreground">
                {t('appearance.options.system.description')}
              </span>
            </span>
          </span>
          {active === 'system' ? (
            <span
              aria-hidden
              className="inline-flex h-6 w-6 items-center justify-center rounded-full bg-primary text-xs font-semibold text-primary-foreground"
            >
              ✓
            </span>
          ) : (
            <span
              aria-hidden
              className="inline-flex h-6 w-6 items-center justify-center rounded-full border border-border/60 text-xs text-muted-foreground"
            >
              ○
            </span>
          )}
        </button>
      </div>

      <div className="min-h-[1.5rem] text-sm">
        {isPending ? (
          <span className="text-muted-foreground">
            {t('appearance.status.saving')}
          </span>
        ) : errorMessage ? (
          <span className="text-destructive">
            {t('appearance.status.couldNotSave', {
              error: errorMessage,
            })}
          </span>
        ) : isUsingPreference && active !== 'system' ? (
          <span className="text-muted-foreground">
            {t('appearance.status.usingPreference', {
              appearance:
                active === 'dark'
                  ? t('appearance.options.dark.label')
                  : t('appearance.options.light.label'),
            })}
          </span>
        ) : (
          <span className="text-muted-foreground">
            {t('appearance.status.usingSystem', {
              appearance: effectiveLabel,
            })}
          </span>
        )}
      </div>
    </div>
  )
}
