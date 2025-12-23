'use client'

import { startTransition, useEffect, useRef, useState } from 'react'
import { useTranslations } from 'next-intl'
import type { BackendApiError } from '@/lib/errors'

export interface ToastMessage {
  id: string
  message: string
  type: 'success' | 'error' | 'warning'
  error?: BackendApiError
}

interface ToastProps {
  toasts: ToastMessage[]
  onClose: (id: string) => void
}

export default function Toast({ toasts, onClose }: ToastProps) {
  if (toasts.length === 0) return null

  return (
    <div className="fixed bottom-4 right-4 z-[100] flex max-w-md flex-col gap-2">
      {toasts.map((toast) => (
        <ToastItem
          key={toast.id}
          toast={toast}
          onClose={() => onClose(toast.id)}
        />
      ))}
    </div>
  )
}

function ToastItem({
  toast,
  onClose,
}: {
  toast: ToastMessage
  onClose: () => void
}) {
  const t = useTranslations('errors.toast')
  const [expanded, setExpanded] = useState(false)
  const [isExiting, setIsExiting] = useState(false)
  const exitTimerRef = useRef<NodeJS.Timeout | null>(null)

  useEffect(() => {
    // Use startTransition to mark as non-urgent update to avoid cascading renders
    startTransition(() => {
      setExpanded(false)
    })
    // Auto-dismiss all toast types after 3 seconds
    const timer = setTimeout(() => {
      setIsExiting(true)
      exitTimerRef.current = setTimeout(() => {
        onClose()
      }, 1000)
    }, 3000)
    return () => {
      clearTimeout(timer)
      if (exitTimerRef.current) {
        clearTimeout(exitTimerRef.current)
        exitTimerRef.current = null
      }
    }
  }, [toast, onClose])

  const handleClose = () => {
    if (isExiting) return
    setIsExiting(true)
    exitTimerRef.current = setTimeout(() => {
      onClose()
    }, 1000)
  }

  const isError = toast.type === 'error'
  const isWarning = toast.type === 'warning'
  const hasTraceId = toast.error?.traceId

  return (
    <div
      className={`rounded-lg border p-4 shadow-elevated transition-opacity duration-1000 ${
        isExiting ? 'opacity-0' : 'opacity-100'
      } ${
        isError
          ? 'border-danger/40 bg-danger/10'
          : isWarning
            ? 'border-orange-500/60 bg-orange-500/20'
            : 'border-success/40 bg-success/10'
      }`}
    >
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <p
            className={`text-sm font-medium ${
              isError
                ? 'text-danger'
                : isWarning
                  ? '[color:color-mix(in_srgb,var(--color-warning)_75%,var(--color-warning-contrast)_25%)]'
                  : '[color:color-mix(in_srgb,var(--color-success)_75%,var(--color-success-contrast)_25%)]'
            }`}
          >
            {toast.message}
          </p>
          {isError && toast.error && (
            <div className="mt-2">
              {expanded && (
                <div className="space-y-1 text-xs text-danger/80">
                  <p>
                    <span className="font-semibold">
                      {t('details.statusLabel')}:
                    </span>{' '}
                    {toast.error.status}
                  </p>
                  {toast.error.code && (
                    <p>
                      <span className="font-semibold">
                        {t('details.codeLabel')}:
                      </span>{' '}
                      {toast.error.code}
                    </p>
                  )}
                  {hasTraceId && (
                    <p className="font-mono text-xs break-all">
                      <span className="font-semibold">
                        {t('details.traceIdLabel')}:
                      </span>{' '}
                      {toast.error.traceId}
                    </p>
                  )}
                </div>
              )}
              {hasTraceId && (
                <button
                  onClick={() => setExpanded(!expanded)}
                  className="mt-1 text-xs text-danger/90 hover:text-danger underline"
                >
                  {expanded ? t('details.hide') : t('details.show')}{' '}
                  {t('details.details')}
                </button>
              )}
            </div>
          )}
        </div>
        <button
          onClick={handleClose}
          className={`ml-4 text-sm font-semibold transition-colors ${
            isError
              ? 'text-danger hover:text-danger/80'
              : isWarning
                ? '[color:color-mix(in_srgb,var(--color-warning)_75%,var(--color-warning-contrast)_25%)] hover:[color:color-mix(in_srgb,var(--color-warning)_60%,var(--color-warning-contrast)_40%)]'
                : '[color:color-mix(in_srgb,var(--color-success)_75%,var(--color-success-contrast)_25%)] hover:[color:color-mix(in_srgb,var(--color-success)_60%,var(--color-success-contrast)_40%)]'
          }`}
          aria-label={t('closeAria')}
        >
          ×
        </button>
      </div>
    </div>
  )
}
