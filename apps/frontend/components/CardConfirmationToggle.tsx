'use client'

import { useState, useTransition } from 'react'
import { useTranslations } from 'next-intl'
import { updateUserOptionsAction } from '@/app/actions/settings-actions'

interface CardConfirmationToggleProps {
  initialEnabled: boolean
}

export function CardConfirmationToggle({
  initialEnabled,
}: CardConfirmationToggleProps) {
  const t = useTranslations('settings')
  const [enabled, setEnabled] = useState(initialEnabled)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [isPending, startTransition] = useTransition()

  const handleToggle = () => {
    const nextValue = !enabled
    setEnabled(nextValue)
    setErrorMessage(null)

    startTransition(async () => {
      const result = await updateUserOptionsAction({
        require_card_confirmation: nextValue,
      })

      if (result.kind === 'error') {
        setEnabled(!nextValue)
        setErrorMessage(result.message)
      } else {
        setErrorMessage(null)
      }
    })
  }

  return (
    <div className="flex flex-col gap-3">
      <button
        type="button"
        role="switch"
        aria-checked={enabled}
        onClick={handleToggle}
        disabled={isPending}
        className={`flex items-center justify-between rounded-2xl border px-4 py-3 transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary ${
          enabled
            ? 'border-primary/60 bg-primary/10 text-foreground shadow-inner shadow-primary/20'
            : 'border-border/60 bg-surface/80 text-muted hover:border-primary/40 hover:text-foreground'
        } ${isPending ? 'opacity-80' : ''}`}
      >
        <div className="flex flex-col text-left">
          <span className="text-sm font-semibold text-foreground">
            {t(
              enabled
                ? 'cardConfirmation.toggle.enabled.title'
                : 'cardConfirmation.toggle.disabled.title'
            )}
          </span>
          <span className="text-xs text-subtle">
            {t(
              enabled
                ? 'cardConfirmation.toggle.enabled.description'
                : 'cardConfirmation.toggle.disabled.description'
            )}
          </span>
        </div>
        <span
          className={`inline-flex h-6 w-12 items-center rounded-full border p-0.5 transition ${
            enabled
              ? 'border-primary/60 bg-primary/20'
              : 'border-border/60 bg-border/30'
          }`}
        >
          <span
            className={`inline-block h-5 w-5 rounded-full bg-foreground transition transform ${
              enabled ? 'translate-x-[22px]' : ''
            }`}
          />
        </span>
      </button>
      <div className="min-h-[1.5rem] text-sm">
        {isPending ? (
          <span className="text-muted">
            {t('cardConfirmation.status.saving')}
          </span>
        ) : errorMessage ? (
          <span className="text-danger">
            {t('cardConfirmation.status.couldNotSave', { error: errorMessage })}
          </span>
        ) : (
          <span className="text-subtle">
            {t('cardConfirmation.status.saved')}
          </span>
        )}
      </div>
    </div>
  )
}
