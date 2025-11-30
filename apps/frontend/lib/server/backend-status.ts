// Server-only module to track backend availability state
// This file must never be imported by client code

// Track startup time and backend availability
let startupTime: number | null = null
let backendHasBeenUp: boolean = false

const STARTUP_WINDOW_MS = 30_000 // 30 seconds

/**
 * Checks if we're currently in the startup window.
 * Initializes startup time on first call.
 */
export function isInStartupWindow(): boolean {
  if (!startupTime) {
    startupTime = Date.now()
  }
  return Date.now() - startupTime < STARTUP_WINDOW_MS
}

/**
 * Marks the backend as having been up (successfully connected).
 * This helps differentiate between startup failures and runtime failures.
 */
export function markBackendUp(): void {
  backendHasBeenUp = true
}

/**
 * Checks if a failure represents a runtime failure (backend was up, now down)
 * vs a startup failure (backend never been up).
 */
export function isRuntimeFailure(): boolean {
  return backendHasBeenUp && !isInStartupWindow()
}

/**
 * Checks if we should log an error based on current state.
 * Errors are only logged if:
 * - We're outside the startup window, OR
 * - Backend was previously up (runtime failure)
 */
export function shouldLogError(): boolean {
  return !isInStartupWindow() || isRuntimeFailure()
}
