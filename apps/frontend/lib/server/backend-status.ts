// Server-only module to track backend availability state.
// This file must never be imported by client code.

/** Service modes mirroring backend conventions. */
export type FrontendServiceMode = 'startup' | 'healthy' | 'recovering'

// ── Internal state ──────────────────────────────────────────────────

let mode: FrontendServiceMode = 'startup'
let consecutiveSuccesses = 0
let consecutiveFailures = 0
let lastOk: number | null = null
let lastError: string | null = null

const FAILURE_THRESHOLD = 2
const RECOVERY_THRESHOLD = 2

// ── Queries ─────────────────────────────────────────────────────────

export function isBackendReady(): boolean {
  return mode === 'healthy'
}

export function getBackendMode(): FrontendServiceMode {
  return mode
}

export function getBackendStatus() {
  return {
    consecutiveSuccesses,
    consecutiveFailures,
    lastOk,
    lastError,
  }
}

/**
 * Whether we should log an error for a given failure.
 * We log the first failure (WARN-level externally) and sustained failures.
 */
export function shouldLogError(): boolean {
  return mode !== 'startup' || consecutiveFailures > 0
}

// ── Mutations ───────────────────────────────────────────────────────

/**
 * Mark a successful backend check.
 */
export function markBackendUp(): void {
  consecutiveSuccesses += 1
  consecutiveFailures = 0
  lastOk = Date.now()

  if (mode === 'startup' || mode === 'recovering') {
    if (consecutiveSuccesses >= RECOVERY_THRESHOLD) {
      const previous = mode
      mode = 'healthy'
      console.log(
        `[readiness] frontend mode: ${previous} → healthy (backend reachable)`
      )
    }
  }
}

/**
 * Mark a failed backend check.
 */
export function markBackendDown(error?: string): void {
  consecutiveFailures += 1
  consecutiveSuccesses = 0
  if (error) lastError = error

  if (mode === 'healthy') {
    if (consecutiveFailures >= FAILURE_THRESHOLD) {
      mode = 'recovering'
      console.error(
        `[readiness] frontend mode: healthy → recovering (backend unreachable: ${error ?? 'unknown'})`
      )
    } else if (consecutiveFailures === 1) {
      console.warn(
        `[readiness] first backend failure detected: ${error ?? 'unknown'}`
      )
    }
  }
}
