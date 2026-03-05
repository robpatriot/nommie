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
let statusVersion = 0

const FAILURE_THRESHOLD = 2
const RECOVERY_THRESHOLD = 2

// ── Queries ─────────────────────────────────────────────────────────

export function isBackendReady(): boolean {
  return mode === 'healthy'
}

/**
 * True when we have concluded the backend is down.
 * Covers both the recovering state (was healthy, now failing) and the startup
 * state where at least one failure has been recorded (backend unreachable from
 * boot). Use this to decide whether to tell the client to show the degraded
 * banner.
 */
export function isBackendKnownDown(): boolean {
  return (
    mode === 'recovering' || (mode === 'startup' && consecutiveFailures > 0)
  )
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
    statusVersion,
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
export function markBackendUp(source?: string): void {
  statusVersion += 1
  void source
  consecutiveSuccesses += 1
  consecutiveFailures = 0
  lastOk = Date.now()

  if (mode === 'startup' || mode === 'recovering') {
    if (consecutiveSuccesses >= RECOVERY_THRESHOLD) {
      mode = 'healthy'
    }
  }
}

/**
 * Mark a failed backend check.
 */
export function markBackendDown(error?: string, source?: string): void {
  statusVersion += 1
  void source
  consecutiveFailures += 1
  consecutiveSuccesses = 0
  if (error) lastError = error

  if (mode === 'healthy') {
    if (consecutiveFailures >= FAILURE_THRESHOLD) {
      mode = 'recovering'
    }
  }
}
