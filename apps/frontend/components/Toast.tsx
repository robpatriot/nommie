'use client'

import { startTransition, useEffect, useState } from 'react'
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

  // Sort toasts so warnings always appear at the top (to prevent UI shifting when
  // warnings flash on/off), followed by errors, then success messages.
  // This ensures errors stay in a stable position even when warnings toggle.
  // Note: flex-col-reverse means the last item in the array appears at the top visually,
  // so we sort warnings LAST in the array to make them appear at the TOP visually.
  const sortedToasts = [...toasts].sort((a, b) => {
    // Warnings last (highest priority) - will appear at top due to flex-col-reverse
    if (a.type === 'warning' && b.type !== 'warning') return 1
    if (a.type !== 'warning' && b.type === 'warning') return -1
    // Errors second (will appear in middle)
    if (a.type === 'error' && b.type !== 'error') return 1
    if (a.type !== 'error' && b.type === 'error') return -1
    // Success messages first (will appear at bottom)
    // If same type, maintain original order
    return 0
  })

  return (
    <div className="fixed bottom-4 right-4 z-[100] flex max-w-md flex-col-reverse gap-2">
      {sortedToasts.map((toast) => (
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

  useEffect(() => {
    // Use startTransition to mark as non-urgent update to avoid cascading renders
    startTransition(() => {
      setExpanded(false)
    })
    // Auto-dismiss success messages after 3 seconds
    if (toast.type === 'success') {
      const timer = setTimeout(() => {
        onClose()
      }, 3000)
      return () => clearTimeout(timer)
    }
  }, [toast, onClose])

  const isError = toast.type === 'error'
  const isWarning = toast.type === 'warning'
  const hasTraceId = toast.error?.traceId

  return (
    <div
      className={`rounded-lg border p-4 shadow-elevated ${
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
                  ? 'text-warning-foreground dark:text-warning'
                  : 'text-success-foreground dark:text-success-contrast'
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
          onClick={onClose}
          className={`ml-4 text-sm font-semibold transition-colors ${
            isError
              ? 'text-danger hover:text-danger/80'
              : isWarning
                ? 'text-warning-foreground dark:text-warning hover:text-warning-foreground/80 dark:hover:text-warning/80'
                : 'text-success-foreground dark:text-success-contrast hover:text-success-foreground/80 dark:hover:text-success-contrast/80'
          }`}
          aria-label={t('closeAria')}
        >
          ×
        </button>
      </div>
    </div>
  )
}
