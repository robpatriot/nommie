// Server-only utility to check backend readiness.
// This file must never be imported by client code.

import { getBackendBaseUrlOrThrow } from '@/auth'
import {
  markBackendUp,
  markBackendDown,
  shouldLogError,
} from './backend-status'
import { isBackendConnectionError } from './connection-errors'
import { logError } from '@/lib/logging/error-logger'

export interface BackendReadinessResult {
  ready: boolean
  error?: string
}

/**
 * Check if the backend is ready by hitting `/api/readyz`.
 *
 * Updates the server-side readiness state on success/failure.
 */
export async function checkBackendReadiness(): Promise<BackendReadinessResult> {
  try {
    const backendBase = getBackendBaseUrlOrThrow()
    const base = backendBase.replace(/\/$/, '')
    const response = await fetch(`${base}/api/readyz`, {
      method: 'GET',
      headers: { 'Content-Type': 'application/json' },
      signal: AbortSignal.timeout(5000),
    })

    if (response.ok) {
      markBackendUp()
      return { ready: true }
    }

    // Got a response (e.g. 503) – backend is alive but not ready
    const errorMsg = `Backend not ready (HTTP ${response.status})`
    markBackendDown(errorMsg)
    return { ready: false, error: errorMsg }
  } catch (error) {
    const isConnection = isBackendConnectionError(error)
    const errorMsg = error instanceof Error ? error.message : 'Unknown error'

    markBackendDown(errorMsg)

    if (shouldLogError() && isConnection) {
      logError('Backend readiness check failed (connection error)', error)
    } else if (shouldLogError() && !isConnection) {
      logError('Backend readiness check failed', error)
    }

    return { ready: false, error: errorMsg }
  }
}
