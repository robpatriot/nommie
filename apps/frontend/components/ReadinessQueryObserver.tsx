'use client'

import { useEffect } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useBackendReadiness } from '@/lib/providers/backend-readiness-provider'
import { BackendApiError } from '@/lib/errors'
import { isNetworkError } from '@/lib/retry'

/**
 * Type guard for QueryCache/MutationCache "updated" events with action.type === 'error'.
 * Matches @tanstack/query-core NotifyEventQueryUpdated / NotifyEventMutationUpdated + ErrorAction.
 */
function isCacheErrorEvent(
  event: unknown
): event is { type: 'updated'; action: { type: 'error'; error: unknown } } {
  return (
    typeof event === 'object' &&
    event !== null &&
    'type' in event &&
    (event as { type: string }).type === 'updated' &&
    'action' in event &&
    typeof (event as { action: unknown }).action === 'object' &&
    (event as { action: { type: string } }).action !== null &&
    (event as { action: { type: string } }).action.type === 'error' &&
    'error' in (event as { action: object }).action
  )
}

/**
 * Type guard for QueryCache/MutationCache "updated" events with action.type === 'success'.
 * Matches @tanstack/query-core NotifyEventQueryUpdated / NotifyEventMutationUpdated + SuccessAction.
 */
function isCacheSuccessEvent(
  event: unknown
): event is { type: 'updated'; action: { type: 'success' } } {
  return (
    typeof event === 'object' &&
    event !== null &&
    'type' in event &&
    (event as { type: string }).type === 'updated' &&
    'action' in event &&
    typeof (event as { action: unknown }).action === 'object' &&
    (event as { action: { type: string } }).action !== null &&
    (event as { action: { type: string } }).action.type === 'success'
  )
}

/**
 * Subscribes to React Query's QueryCache and MutationCache and reports backend
 * failures to BackendReadinessProvider (reportFailure). Permanent errors (503,
 * 502, 504, network) enter degraded immediately; others require 2 consecutive
 * failures. Idempotent: multiple failures while already degraded do not create
 * multiple polling loops (provider handles that).
 * reportSuccess() only resets transient-failure count; recovery to healthy is
 * driven solely by the /readyz polling loop (2 consecutive probe successes).
 */
export default function ReadinessQueryObserver() {
  const { reportDependencyOutage, reportOperationSuccess } =
    useBackendReadiness()
  const queryClient = useQueryClient()

  useEffect(() => {
    const queryCache = queryClient.getQueryCache()
    const unsubQuery = queryCache.subscribe((event) => {
      if (!isCacheErrorEvent(event)) return
      const error = event.action.error
      // Network errors are treated as client connectivity problems and do not
      // assert backend dependency failure.
      if (isNetworkError(error)) return
      if (
        error instanceof BackendApiError &&
        error.code === 'SERVICE_UNAVAILABLE'
      ) {
        reportDependencyOutage()
      }
    })
    const mutationCache = queryClient.getMutationCache()
    const unsubMutation = mutationCache.subscribe((event) => {
      if (!isCacheErrorEvent(event)) return
      const error = event.action.error
      if (isNetworkError(error)) return
      if (
        error instanceof BackendApiError &&
        error.code === 'SERVICE_UNAVAILABLE'
      ) {
        reportDependencyOutage()
      }
    })
    return () => {
      unsubQuery()
      unsubMutation()
    }
  }, [queryClient, reportDependencyOutage])

  useEffect(() => {
    const queryCache = queryClient.getQueryCache()
    const unsubQuery = queryCache.subscribe((event) => {
      if (!isCacheSuccessEvent(event)) return
      reportOperationSuccess()
    })
    const mutationCache = queryClient.getMutationCache()
    const unsubMutation = mutationCache.subscribe((event) => {
      if (!isCacheSuccessEvent(event)) return
      reportOperationSuccess()
    })
    return () => {
      unsubQuery()
      unsubMutation()
    }
  }, [queryClient, reportOperationSuccess])

  return null
}
