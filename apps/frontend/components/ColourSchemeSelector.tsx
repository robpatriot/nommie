'use client'

import { useMemo } from 'react'
import { useTranslations } from 'next-intl'
import { COLOUR_SCHEMES, useTheme, type ColourScheme } from './theme-provider'

const SPECIFIC_OPTIONS: Array<{
  value: Extract<ColourScheme, 'light' | 'dark'>
  emoji: string
}> = [
  { value: 'light', emoji: '🌞' },
  { value: 'dark', emoji: '🌙' },
]

// Optional sanity: ensure the provider’s list still contains what this UI expects
void COLOUR_SCHEMES

export function ColourSchemeSelector({
  preferredColourScheme,
}: {
  preferredColourScheme: ColourScheme | null
}) {
  const t = useTranslations('settings')

  const {
    colourScheme,
    setColourScheme,
    resolvedColourScheme,
    hydrated,
    isSaving,
    errorMessage,
    clearError,
  } = useTheme()

  // Before hydration, show backend preference (server-provided).
  // After hydration, ThemeProvider is authoritative.
  const active = useMemo<ColourScheme>(() => {
    if (hydrated) return colourScheme
    return preferredColourScheme ?? 'system'
  }, [hydrated, colourScheme, preferredColourScheme])

  // Matches your previous UX logic:
  // "Using preference" = backend had explicit light/dark (not null, not system)
  const isUsingPreference =
    preferredColourScheme !== null && preferredColourScheme !== 'system'

  const effectiveLabel =
    resolvedColourScheme === 'dark'
      ? t('colour_scheme.options.dark.label')
      : t('colour_scheme.options.light.label')

  const handleSelect = (mode: ColourScheme) => {
    if (hydrated && mode === colourScheme) return
    clearError()
    void setColourScheme(mode)
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-col gap-2">
        {/* Specific colour_scheme options */}
        {SPECIFIC_OPTIONS.map((option) => {
          const isActive = active === option.value
          return (
            <button
              key={option.value}
              type="button"
              onClick={() => handleSelect(option.value)}
              disabled={isSaving}
              aria-pressed={isActive}
              className={`flex items-center justify-between rounded-2xl border px-4 py-3 text-left transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary ${
                isActive
                  ? 'border-primary/60 bg-primary/10 text-foreground shadow-inner shadow-primary/20'
                  : 'border-border/60 bg-card/80 text-muted-foreground hover:border-primary/40 hover:text-foreground'
              } ${isSaving ? 'opacity-80' : ''}`}
            >
              <span className="flex items-center gap-3">
                <span aria-hidden className="text-xl">
                  {option.emoji}
                </span>
                <span className="flex flex-col">
                  <span className="text-sm font-semibold text-foreground">
                    {t(`colour_scheme.options.${option.value}.label`)}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    {t(`colour_scheme.options.${option.value}.description`)}
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
            {t('colour_scheme.separator')}
          </span>
          <div className="h-px flex-1 bg-border/30" />
        </div>

        {/* System default option */}
        <button
          type="button"
          onClick={() => handleSelect('system')}
          disabled={isSaving}
          aria-pressed={active === 'system'}
          className={`flex items-center justify-between rounded-2xl border px-4 py-3 text-left transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary ${
            active === 'system'
              ? 'border-primary/60 bg-primary/10 text-foreground shadow-inner shadow-primary/20'
              : 'border-dashed border-muted/40 bg-card/40 text-muted-foreground hover:border-primary/40 hover:bg-card/60 hover:text-foreground'
          } ${isSaving ? 'opacity-80' : ''}`}
        >
          <span className="flex items-center gap-3">
            <span aria-hidden className="text-xl">
              🖥️
            </span>
            <span className="flex flex-col">
              <span className="text-sm font-semibold text-foreground">
                {t('colour_scheme.options.system.label')}
              </span>
              <span className="text-xs text-muted-foreground">
                {t('colour_scheme.options.system.description')}
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
        {isSaving ? (
          <span className="text-muted-foreground">
            {t('colour_scheme.status.saving')}
          </span>
        ) : errorMessage ? (
          <span className="text-destructive">
            {t('colour_scheme.status.couldNotSave', {
              error: errorMessage,
            })}
          </span>
        ) : isUsingPreference && active !== 'system' ? (
          <span className="text-muted-foreground">
            {t('colour_scheme.status.usingPreference', {
              colour_scheme:
                active === 'dark'
                  ? t('colour_scheme.options.dark.label')
                  : t('colour_scheme.options.light.label'),
            })}
          </span>
        ) : (
          <span className="text-muted-foreground">
            {t('colour_scheme.status.usingSystem', {
              colour_scheme: effectiveLabel,
            })}
          </span>
        )}
      </div>
    </div>
  )
}
