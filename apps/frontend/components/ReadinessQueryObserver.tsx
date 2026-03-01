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
  const { reportFailure, reportSuccess } = useBackendReadiness()
  const queryClient = useQueryClient()

  useEffect(() => {
    const queryCache = queryClient.getQueryCache()
    const unsubQuery = queryCache.subscribe((event) => {
      if (!isCacheErrorEvent(event)) return
      reportFailure(classifyFailure(event.action.error))
    })
    const mutationCache = queryClient.getMutationCache()
    const unsubMutation = mutationCache.subscribe((event) => {
      if (!isCacheErrorEvent(event)) return
      reportFailure(classifyFailure(event.action.error))
    })
    return () => {
      unsubQuery()
      unsubMutation()
    }
  }, [queryClient, reportFailure])

  useEffect(() => {
    const queryCache = queryClient.getQueryCache()
    const unsubQuery = queryCache.subscribe((event) => {
      if (!isCacheSuccessEvent(event)) return
      reportSuccess()
    })
    const mutationCache = queryClient.getMutationCache()
    const unsubMutation = mutationCache.subscribe((event) => {
      if (!isCacheSuccessEvent(event)) return
      reportSuccess()
    })
    return () => {
      unsubQuery()
      unsubMutation()
    }
  }, [queryClient, reportSuccess])

  return null
}
