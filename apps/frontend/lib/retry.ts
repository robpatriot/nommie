/**
 * Utility functions for retrying network operations with exponential backoff.
 */

/**
 * Determines if an error is a network error that should be retried.
 * Network errors are typically temporary and worth retrying.
 *
 * @param error - The error to check
 * @returns true if the error is a network error that should be retried
 */
export function isNetworkError(error: unknown): boolean {
  if (!(error instanceof Error)) {
    return false
  }

  const message = error.message.toLowerCase()
  const name = error.name.toLowerCase()

  // Check for common network error patterns
  // - fetch failed (Node.js fetch)
  // - network timeout
  // - network error
  // - failed to fetch (browser fetch)
  // - connection error
  // - ECONNREFUSED, ETIMEDOUT, ENOTFOUND (Node.js network errors)
  const networkErrorPatterns = [
    'fetch failed',
    'network timeout',
    'network error',
    'failed to fetch',
    'connection error',
    'econnrefused',
    'etimedout',
    'enotfound',
    'econnreset',
    'socket hang up',
  ]

  return (
    networkErrorPatterns.some((pattern) => message.includes(pattern)) ||
    name.includes('network') ||
    name.includes('timeout')
  )
}
