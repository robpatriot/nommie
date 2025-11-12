'use client'

import { useCallback, useState } from 'react'
import type { ToastMessage } from '@/components/Toast'
import { BackendApiError } from '@/lib/errors'

/**
 * Hook for managing toast notifications.
 * Provides a consistent API for showing success and error toasts.
 *
 * @returns An object with toast state and show function
 */
export function useToast() {
  const [toast, setToast] = useState<ToastMessage | null>(null)

  const showToast = useCallback(
    (message: string, type: ToastMessage['type'], error?: BackendApiError) => {
      setToast({
        id: Date.now().toString(),
        message,
        type,
        error,
      })
    },
    []
  )

  const hideToast = useCallback(() => {
    setToast(null)
  }, [])

  return {
    toast,
    showToast,
    hideToast,
  }
}
