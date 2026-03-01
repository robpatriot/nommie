// Server-only utility to check backend readiness.
// This file must never be imported by client code.

import { getBackendBaseUrlOrThrow } from '@/auth'
import {
  markBackendUp,
  markBackendDown,
  getBackendMode,
  getBackendStatus,
} from './backend-status'

export interface BackendReadinessResult {
  ready: boolean
  error?: string
}

const PROBE_TIMEOUT_MS = 1_000
const READINESS_RETRY_AFTER_MS = 30_000

let lastBackendAttemptAt = 0

/**
 * True probe used by FE /readyz: always hits the backend with a 1s timeout.
 * Does not use cached "known down" state, so client polling can detect recovery promptly.
 */
export async function probeBackendReadiness(): Promise<BackendReadinessResult> {
  try {
    const backendBase = getBackendBaseUrlOrThrow()
    const base = backendBase.replace(/\/$/, '')
    const response = await fetch(`${base}/api/readyz`, {
      method: 'GET',
      headers: { 'Content-Type': 'application/json' },
      signal: AbortSignal.timeout(PROBE_TIMEOUT_MS),
    })

    if (response.ok) {
      return { ready: true }
    }
    return {
      ready: false,
      error: `Backend not ready (HTTP ${response.status})`,
    }
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : 'Unknown error'
    return { ready: false, error: errorMsg }
  }
}

/**
 * SSR/RSC path: checks backend readiness with backoff when backend is known down
 * (avoids hammering the backend). Updates server-side readiness state.
 * Use this for layout, refresh-backend-jwt, etc. Do NOT use for FE /readyz polling.
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

    const errorMsg = `Backend not ready (HTTP ${response.status})`
    markBackendDown(errorMsg)
    return { ready: false, error: errorMsg }
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : 'Unknown error'
    markBackendDown(errorMsg)
    return { ready: false, error: errorMsg }
  }
}
