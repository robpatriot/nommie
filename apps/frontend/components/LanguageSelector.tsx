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
  preferredLocale: string | null
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

  const initialSelection = useMemo<SupportedLocale>(() => {
    if (preferredLocale && isSupportedLocale(preferredLocale)) {
      return preferredLocale
    }
    return effectiveLocale
  }, [preferredLocale, effectiveLocale])

  const [selected, setSelected] = useState<SupportedLocale>(initialSelection)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [isPending, startTransition] = useTransition()

  const isUsingPreference = preferredLocale != null
  const selectedLabel = t(`settings.language.options.${selected}.label`)
  const effectiveLabel = t(`settings.language.options.${effectiveLocale}.label`)

  const onChange = (nextLocale: SupportedLocale) => {
    setSelected(nextLocale)
    setErrorMessage(null)

    startTransition(async () => {
      const result = await updateLocaleAction(nextLocale)
      if (result.kind === 'error') {
        setErrorMessage(result.message)
        setSelected(initialSelection)
        return
      }

      router.refresh()
    })
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-col gap-2">
        {SUPPORTED_LOCALES.map((locale) => {
          const isActive = selected === locale
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
                  : 'border-border/60 bg-surface/80 text-muted hover:border-primary/40 hover:text-foreground'
              } ${isPending ? 'opacity-80' : ''}`}
            >
              <span className="flex flex-col">
                <span className="text-sm font-semibold text-foreground">
                  {t(`settings.language.options.${locale}.label`)}
                </span>
                <span className="text-xs text-subtle">
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
          <span className="text-muted">
            {t('settings.language.status.saving')}
          </span>
        ) : errorMessage ? (
          <span className="text-danger">
            {t('settings.language.status.couldNotSave', {
              error: errorMessage,
            })}
          </span>
        ) : isUsingPreference ? (
          <span className="text-subtle">
            {t('settings.language.status.usingPreference', {
              language: selectedLabel,
            })}
          </span>
        ) : (
          <span className="text-subtle">
            {t('settings.language.status.usingBrowser', {
              language: effectiveLabel,
            })}
          </span>
        )}
      </div>
    </div>
  )
}
