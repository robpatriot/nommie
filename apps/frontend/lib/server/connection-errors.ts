// Server-only utility for detecting backend connection errors
// This file must never be imported by client code

/**
 * Determines if an error is a backend connection error.
 * These errors typically indicate the backend is not available (starting up, down, etc.)
 *
 * @param error - The error to check
 * @returns true if the error indicates a connection problem
 */
export function isBackendConnectionError(error: unknown): boolean {
  if (!(error instanceof Error)) {
    return false
  }

  const errorMessage = error.message.toLowerCase()
  const causeMessage =
    error.cause instanceof Error ? error.cause.message.toLowerCase() : ''

  const connectionErrorPatterns = [
    'econnrefused',
    'fetch failed',
    'connection',
    'timeout',
    'connect econnrefused',
  ]

  return (
    connectionErrorPatterns.some((pattern) => errorMessage.includes(pattern)) ||
    connectionErrorPatterns.some((pattern) => causeMessage.includes(pattern))
  )
}

/**
 * Determines if an error represents a backend startup scenario.
 * This is true when we're in the startup window AND it's a connection error.
 *
 * @param error - The error to check
 * @param isInStartupWindow - Function to check if we're in startup window
 * @returns true if this is likely a startup error
 */
export function isBackendStartupError(
  error: unknown,
  isInStartupWindow: () => boolean
): boolean {
  return isInStartupWindow() && isBackendConnectionError(error)
}
