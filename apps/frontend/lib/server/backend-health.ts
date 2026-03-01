// Server-only utility to check backend readiness.
// This file must never be imported by client code.

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
 * Resolves the backend base URL for the readiness probe. When BACKEND_BASE_URL or
 * NEXT_PUBLIC_BACKEND_BASE_URL is set, uses that; otherwise uses same-origin (caller must pass).
 * Never throws.
 */
function getProbeBaseUrl(
  sameOriginFallback: string | undefined
): string | undefined {
  const url =
    process.env.BACKEND_BASE_URL || process.env.NEXT_PUBLIC_BACKEND_BASE_URL
  if (url) {
    try {
      const parsed = new URL(url)
      if (parsed.protocol === 'http:' || parsed.protocol === 'https:') {
        return url.replace(/\/$/, '')
      }
    } catch {
      // fall through to same-origin
    }
  }
  return sameOriginFallback?.replace(/\/$/, '')
}

/**
 * Probe used by FE /readyz: hits the backend readiness endpoint with a 1s timeout.
 * Does not use cached "known down" state, so client polling can detect recovery promptly.
 *
 * **Origin / base URL:** If BACKEND_BASE_URL or NEXT_PUBLIC_BACKEND_BASE_URL is set, that is used.
 * Otherwise, `sameOriginFallback` must be provided (e.g. `new URL(request.url).origin` from the
 * /readyz route handler). If no env is set and no fallback is passed, returns `{ ready: false }`
 * without throwing. Never throws due to missing env or origin.
 *
 * @param sameOriginFallback - When env vars are unset, this origin is used (required for FE /readyz to work without env).
 */
export async function probeBackendReadiness(
  sameOriginFallback?: string
): Promise<BackendReadinessResult> {
  const base = getProbeBaseUrl(sameOriginFallback)
  if (!base) {
    if (process.env.NODE_ENV === 'development') {
      console.debug(
        '[probeBackendReadiness] No backend base URL (env unset and no sameOriginFallback); returning ready: false'
      )
    }
    return { ready: false, error: 'Backend URL not configured' }
  }

  try {
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
    const backendBase =
      process.env.BACKEND_BASE_URL || process.env.NEXT_PUBLIC_BACKEND_BASE_URL
    if (!backendBase) {
      markBackendDown('Backend URL not configured')
      return { ready: false, error: 'Backend URL not configured' }
    }
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
