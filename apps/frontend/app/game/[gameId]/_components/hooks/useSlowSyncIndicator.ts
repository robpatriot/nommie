import { useEffect, useRef } from 'react'

import type { BackendApiError } from '@/lib/errors'

interface UseSlowSyncIndicatorProps {
  isRefreshing: boolean
  showToast: (
    message: string,
    type: 'warning' | 'error' | 'success',
    error?: BackendApiError
  ) => string
  hideToast: (id: string) => void
}

/**
 * Manages the slow sync indicator toast that appears when refresh takes longer than 1 second.
 */
export function useSlowSyncIndicator({
  isRefreshing,
  showToast,
  hideToast,
}: UseSlowSyncIndicatorProps) {
  const slowSyncToastIdRef = useRef<string | null>(null)

  useEffect(() => {
    if (!isRefreshing) {
      // Hide toast when refresh completes
      if (slowSyncToastIdRef.current) {
        hideToast(slowSyncToastIdRef.current)
        slowSyncToastIdRef.current = null
      }
      return
    }

    const timeoutId = setTimeout(() => {
      const toastId = showToast('Updating game state…', 'warning')
      slowSyncToastIdRef.current = toastId
    }, 1000)

    return () => {
      clearTimeout(timeoutId)
    }
  }, [isRefreshing, showToast, hideToast])
}
