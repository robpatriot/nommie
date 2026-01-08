'use client'

import { useMemo, useState, useTransition } from 'react'
import { useLocale, useTranslations } from 'next-intl'
import { useRouter } from 'next/navigation'

import { updateLocaleAction } from '@/app/actions/settings-actions'
import {
  DEFAULT_LOCALE,
  isSupportedLocale,
  SUPPORTED_LOCALES,
  type SupportedLocale,
} from '@/i18n/locale'

export function LanguageSelector({
  preferredLocale,
}: {
  preferredLocale: SupportedLocale | null
}) {
  const t = useTranslations()
  const router = useRouter()
  const effectiveLocaleRaw = useLocale()

  const effectiveLocale = useMemo<SupportedLocale>(() => {
    if (
      typeof effectiveLocaleRaw === 'string' &&
      isSupportedLocale(effectiveLocaleRaw)
    ) {
      return effectiveLocaleRaw
    }
    return DEFAULT_LOCALE
  }, [effectiveLocaleRaw])

  const isUsingPreference = preferredLocale != null
  const [selectedLocale, setSelectedLocale] = useState<SupportedLocale | null>(
    preferredLocale && isSupportedLocale(preferredLocale)
      ? preferredLocale
      : null
  )
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [isPending, startTransition] = useTransition()

  const selectedLabel = selectedLocale
    ? t(`settings.language.options.${selectedLocale}.label`)
    : t('settings.language.options.browser.label')
  const effectiveLabel = t(`settings.language.options.${effectiveLocale}.label`)

  const onChange = (nextLocale: SupportedLocale | null) => {
    setSelectedLocale(nextLocale)
    setErrorMessage(null)

    startTransition(async () => {
      const result = await updateLocaleAction(nextLocale)
      if (result.kind === 'error') {
        setErrorMessage(result.message)
        setSelectedLocale(
          preferredLocale && isSupportedLocale(preferredLocale)
            ? preferredLocale
            : null
        )
        return
      }

      router.refresh()
    })
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-col gap-2">
        {/* Specific locale options */}
        {SUPPORTED_LOCALES.map((locale) => {
          const isActive = selectedLocale === locale
          return (
            <button
              key={locale}
              type="button"
              onClick={() => onChange(locale)}
              disabled={isPending}
              aria-pressed={isActive}
              className={`flex items-center justify-between rounded-2xl border px-4 py-3 text-left transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary ${
                isActive
                  ? 'border-primary/60 bg-primary/10 text-foreground shadow-inner shadow-primary/20'
                  : 'border-border/60 bg-card/80 text-muted-foreground hover:border-primary/40 hover:text-foreground'
              } ${isPending ? 'opacity-80' : ''}`}
            >
              <span className="flex flex-col">
                <span className="text-sm font-semibold text-foreground">
                  {t(`settings.language.options.${locale}.label`)}
                </span>
                <span className="text-xs text-muted-foreground">
                  {t(`settings.language.options.${locale}.description`)}
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
            {t('settings.language.separator')}
          </span>
          <div className="h-px flex-1 bg-border/30" />
        </div>

        {/* Browser default option */}
        <button
          type="button"
          onClick={() => onChange(null)}
          disabled={isPending}
          aria-pressed={selectedLocale === null}
          className={`flex items-center justify-between rounded-2xl border px-4 py-3 text-left transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary ${
            selectedLocale === null
              ? 'border-primary/60 bg-primary/10 text-foreground shadow-inner shadow-primary/20'
              : 'border-dashed border-muted/40 bg-card/40 text-muted-foreground hover:border-primary/40 hover:bg-card/60 hover:text-foreground'
          } ${isPending ? 'opacity-80' : ''}`}
        >
          <span className="flex items-center gap-3">
            <span aria-hidden className="text-lg">
              🌐
            </span>
            <span className="flex flex-col">
              <span className="text-sm font-semibold text-foreground">
                {t('settings.language.options.browser.label')}
              </span>
              <span className="text-xs text-muted-foreground">
                {t('settings.language.options.browser.description')}
              </span>
            </span>
          </span>
          {selectedLocale === null ? (
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
            {t('settings.language.status.saving')}
          </span>
        ) : errorMessage ? (
          <span className="text-destructive">
            {t('settings.language.status.couldNotSave', {
              error: errorMessage,
            })}
          </span>
        ) : isUsingPreference && selectedLocale ? (
          <span className="text-muted-foreground">
            {t('settings.language.status.usingPreference', {
              language: selectedLabel,
            })}
          </span>
        ) : (
          <span className="text-muted-foreground">
            {t('settings.language.status.usingBrowser', {
              language: effectiveLabel,
            })}
          </span>
        )}
      </div>
    </div>
  )
}
