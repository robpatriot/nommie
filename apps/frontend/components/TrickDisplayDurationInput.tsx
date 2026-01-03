'use client'

import { useState, useTransition } from 'react'
import { useTranslations } from 'next-intl'
import { updateUserOptionsAction } from '@/app/actions/settings-actions'

const DEFAULT_TRICK_DISPLAY_DURATION_SECONDS = 2.0

interface TrickDisplayDurationInputProps {
  initialValue: number | null
}

export function TrickDisplayDurationInput({
  initialValue,
}: TrickDisplayDurationInputProps) {
  const t = useTranslations('settings')
  const [value, setValue] = useState<string>(
    initialValue === null ? '' : initialValue.toString()
  )
  // Track the last successfully saved value for status display
  const [savedValue, setSavedValue] = useState<number | null>(initialValue)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [isPending, startTransition] = useTransition()

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = e.target.value
    setValue(newValue)
    setErrorMessage(null)
  }

  const saveValue = () => {
    startTransition(async () => {
      // Parse the value: empty string = null (use default), otherwise parse as number
      let parsedValue: number | null = null
      if (value.trim() !== '') {
        const parsed = Number.parseFloat(value.trim())
        if (Number.isNaN(parsed) || parsed < 0) {
          setErrorMessage(t('trickDisplayDuration.validation.invalid'))
          // Reset to previous valid value on validation error
          setValue(savedValue === null ? '' : savedValue.toString())
          return
        }
        parsedValue = parsed
      }

      const result = await updateUserOptionsAction({
        trick_display_duration_seconds: parsedValue,
      })

      if (result.kind === 'error') {
        setErrorMessage(result.message)
        // Reset to previous valid value on error
        setValue(savedValue === null ? '' : savedValue.toString())
      } else {
        setErrorMessage(null)
        // Update saved value so status updates immediately
        setSavedValue(parsedValue)
      }
    })
  }

  const handleBlur = () => {
    saveValue()
  }

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      e.currentTarget.blur()
    }
  }

  // Use savedValue for status display instead of initialValue
  const displayValue =
    savedValue === null ? DEFAULT_TRICK_DISPLAY_DURATION_SECONDS : savedValue
  const isUsingDefault = savedValue === null
  const isDisabled = savedValue === 0

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-col gap-2">
        <label className="flex flex-col gap-2">
          <span className="text-sm font-semibold text-foreground">
            {t('trickDisplayDuration.label')}
          </span>
          <span className="text-xs text-subtle">
            {t('trickDisplayDuration.description')}
          </span>
          <input
            type="text"
            inputMode="decimal"
            value={value}
            onChange={handleChange}
            onBlur={handleBlur}
            onKeyDown={handleKeyDown}
            disabled={isPending}
            placeholder={DEFAULT_TRICK_DISPLAY_DURATION_SECONDS.toString()}
            className={`rounded-2xl border px-4 py-3 text-sm transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary ${
              isPending
                ? 'border-border/40 bg-surface/40 text-muted opacity-60'
                : 'border-border/60 bg-surface/80 text-foreground hover:border-primary/40 focus:border-primary/60'
            }`}
            aria-label={t('trickDisplayDuration.label')}
          />
        </label>
        <div className="text-xs text-subtle">
          {t('trickDisplayDuration.help', {
            defaultValue: DEFAULT_TRICK_DISPLAY_DURATION_SECONDS,
          })}
        </div>
      </div>
      <div className="min-h-[1.5rem] text-sm">
        {isPending ? (
          <span className="text-muted">
            {t('trickDisplayDuration.status.saving')}
          </span>
        ) : errorMessage ? (
          <span className="text-danger">
            {t('trickDisplayDuration.status.couldNotSave', {
              error: errorMessage,
            })}
          </span>
        ) : isUsingDefault ? (
          <span className="text-subtle">
            {t('trickDisplayDuration.status.usingDefault', {
              value: displayValue,
            })}
          </span>
        ) : isDisabled ? (
          <span className="text-subtle">
            {t('trickDisplayDuration.status.disabled')}
          </span>
        ) : (
          <span className="text-subtle">
            {t('trickDisplayDuration.status.usingCustom', {
              value: displayValue,
            })}
          </span>
        )}
      </div>
    </div>
  )
}
