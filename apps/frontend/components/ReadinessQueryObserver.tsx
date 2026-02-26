'use client'

import { useEffect } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useBackendReadiness } from '@/lib/providers/backend-readiness-provider'
import { BackendApiError } from '@/lib/errors'
import { isNetworkError } from '@/lib/retry'

/**
 * Helper to check if a status code indicates an infrastructure failure.
 * 503: Service Unavailable (Backend Gate)
 * 502: Bad Gateway
 * 504: Gateway Timeout
 */
function isInfraFailure(status: number): boolean {
  return status === 503 || status === 502 || status === 504
}

/**
 * Component that bridges TanStack Query errors to the BackendReadinessProvider.
 *
 * It listens to the QueryCache for specific errors (503, 502, 504, and network failures)
 * and triggers the readiness recovery polling loop when they occur.
 */
export default function ReadinessQueryObserver() {
  const { triggerRecovery } = useBackendReadiness()
  const queryClient = useQueryClient()

  useEffect(() => {
    // Subscribe to all changes in the query cache
    const unsubscribe = queryClient.getQueryCache().subscribe((event) => {
      // We only care about queries that transitioned to an error state
      if (event.type === 'updated' && event.action.type === 'error') {
        const error = event.action.error

        let shouldTrigger = false

        if (error instanceof BackendApiError) {
          if (isInfraFailure(error.status)) {
            shouldTrigger = true
          }
        } else if (isNetworkError(error)) {
          shouldTrigger = true
        }

        if (shouldTrigger) {
          triggerRecovery()
        }
      }
    })

    return () => {
      unsubscribe()
    }
  }, [queryClient, triggerRecovery])

  return null
}
