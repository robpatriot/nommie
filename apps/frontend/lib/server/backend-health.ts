// Server-only utility to check backend readiness.
// This file must never be imported by client code.

import { getBackendBaseUrlOrThrow } from '@/auth'
import {
  markBackendUp,
  markBackendDown,
  shouldLogError,
  getBackendMode,
  getBackendStatus,
} from './backend-status'
import { isBackendConnectionError } from './connection-errors'
import { logError } from '@/lib/logging/error-logger'

export interface BackendReadinessResult {
  ready: boolean
  error?: string
}

const READINESS_RETRY_AFTER_MS = 30_000

let lastBackendAttemptAt = 0

/**
 * Check if the backend is ready by hitting `/api/readyz`.
 *
 * Updates the server-side readiness state on success/failure.
 * When we already know the backend is down, returns not ready without calling it,
 * except we try again every READINESS_RETRY_AFTER_MS to discover when it recovers.
 */
export async function checkBackendReadiness(): Promise<BackendReadinessResult> {
  const mode = getBackendMode()
  const { consecutiveFailures } = getBackendStatus()
  const knownDown =
    mode === 'recovering' || (mode === 'startup' && consecutiveFailures > 0)

  if (knownDown) {
    const now = Date.now()
    if (now - lastBackendAttemptAt < READINESS_RETRY_AFTER_MS) {
      return { ready: false, error: 'Backend known down' }
    }
    lastBackendAttemptAt = now
  }

  try {
    lastBackendAttemptAt = Date.now()
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

    if (getBackendMode() === 'healthy' && shouldLogError()) {
      if (isConnection) {
        logError('Backend readiness check failed (connection error)', error)
      } else {
        logError('Backend readiness check failed', error)
      }
    }

    return { ready: false, error: errorMsg }
  }
}
