'use client'

import { useState, useEffect } from 'react'
import { BackendApiError } from '@/lib/errors'

export interface ToastMessage {
  id: string
  message: string
  type: 'success' | 'error'
  error?: BackendApiError
}

interface ToastProps {
  toast: ToastMessage | null
  onClose: () => void
}

export default function Toast({ toast, onClose }: ToastProps) {
  const [expanded, setExpanded] = useState(false)

  useEffect(() => {
    if (toast) {
      setExpanded(false)
      // Auto-dismiss success messages after 3 seconds
      if (toast.type === 'success') {
        const timer = setTimeout(() => {
          onClose()
        }, 3000)
        return () => clearTimeout(timer)
      }
    }
  }, [toast, onClose])

  if (!toast) return null

  const isError = toast.type === 'error'
  const hasTraceId = toast.error?.traceId

  return (
    <div className="fixed bottom-4 right-4 z-50 max-w-md">
      <div
        className={`rounded-lg border p-4 shadow-elevated ${
          isError
            ? 'border-danger/40 bg-danger/10'
            : 'border-success/40 bg-success/10'
        }`}
      >
        <div className="flex items-start justify-between">
          <div className="flex-1">
            <p
              className={`text-sm font-medium ${
                isError ? 'text-danger-foreground' : 'text-success-foreground'
              }`}
            >
              {toast.message}
            </p>
            {isError && toast.error && (
              <div className="mt-2">
                {expanded && (
                  <div className="space-y-1 text-xs text-danger-foreground/80">
                    <p>
                      <span className="font-semibold">Status:</span>{' '}
                      {toast.error.status}
                    </p>
                    {toast.error.code && (
                      <p>
                        <span className="font-semibold">Code:</span>{' '}
                        {toast.error.code}
                      </p>
                    )}
                    {hasTraceId && (
                      <p className="font-mono text-xs break-all">
                        <span className="font-semibold">Trace ID:</span>{' '}
                        {toast.error.traceId}
                      </p>
                    )}
                  </div>
                )}
                {hasTraceId && (
                  <button
                    onClick={() => setExpanded(!expanded)}
                    className="mt-1 text-xs text-danger hover:text-danger-foreground underline"
                  >
                    {expanded ? 'Hide' : 'Show'} details
                  </button>
                )}
              </div>
            )}
          </div>
          <button
            onClick={onClose}
            className={`ml-4 text-sm font-semibold transition-colors ${
              isError
                ? 'text-danger hover:text-danger-foreground'
                : 'text-success hover:text-success-foreground'
            }`}
            aria-label="Close"
          >
            ×
          </button>
        </div>
      </div>
    </div>
  )
}
