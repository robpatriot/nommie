'use client'

import { useEffect } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useBackendReadiness } from '@/lib/providers/backend-readiness-provider'
import type { FailureKind } from '@/lib/providers/backend-readiness-provider'
import { BackendApiError } from '@/lib/errors'
import { isNetworkError } from '@/lib/retry'

function isPermanentFailure(error: unknown): boolean {
  if (error instanceof BackendApiError) {
    if (error.status === 503 || error.status === 502 || error.status === 504) {
      return true
    }
  }
  if (isNetworkError(error)) {
    return true
  }
  return false
}

function classifyFailure(error: unknown): FailureKind {
  return isPermanentFailure(error) ? 'permanent' : 'transient'
}

/**
 * Subscribes to React Query's QueryCache and reports backend failures to
 * BackendReadinessProvider (reportFailure). Permanent errors (503, 502, 504,
 * network) enter degraded immediately; others require 2 consecutive failures.
 */
export default function ReadinessQueryObserver() {
  const { reportFailure, reportSuccess } = useBackendReadiness()
  const queryClient = useQueryClient()

  useEffect(() => {
    const cache = queryClient.getQueryCache()
    const unsubscribe = cache.subscribe((event) => {
      if (event.type !== 'updated' || event.action.type !== 'error') return
      const error = event.action.error
      reportFailure(classifyFailure(error))
    })

    return () => {
      unsubscribe()
    }
  }, [queryClient, reportFailure])

  useEffect(() => {
    const cache = queryClient.getQueryCache()
    const unsubscribe = cache.subscribe((event) => {
      if (event.type !== 'updated' || event.action.type !== 'success') return
      reportSuccess()
    })

    return () => {
      unsubscribe()
    }
  }, [queryClient, reportSuccess])

  return null
}
