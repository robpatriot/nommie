'use client'

import { useCallback, useState } from 'react'
import type { ToastMessage } from '@/components/Toast'
import { BackendApiError } from '@/lib/errors'

/**
 * Hook for managing toast notifications.
 * Provides a consistent API for showing success and error toasts.
 * Toasts are stackable and appear on top of each other.
 *
 * @returns An object with toast state and show function
 */
export function useToast() {
  const [toasts, setToasts] = useState<ToastMessage[]>([])

  const showToast = useCallback(
    (message: string, type: ToastMessage['type'], error?: BackendApiError) => {
      const id = createToastId()
      setToasts((prev) => [
        ...prev,
        {
          id,
          message,
          type,
          error,
        },
      ])
      return id
    },
    []
  )

  const hideToast = useCallback((id?: string) => {
    if (id) {
      // Hide specific toast by id
      setToasts((prev) => prev.filter((toast) => toast.id !== id))
    } else {
      // Hide all toasts
      setToasts([])
    }
  }, [])

  return {
    toasts,
    showToast,
    hideToast,
  }
}

function createToastId() {
  if (
    typeof crypto !== 'undefined' &&
    typeof crypto.randomUUID === 'function'
  ) {
    return crypto.randomUUID()
  }
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`
}
