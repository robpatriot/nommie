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

/**
 * Calculates the delay for exponential backoff retry.
 *
 * @param attempt - The current attempt number (0-indexed)
 * @param baseDelayMs - Base delay in milliseconds (default: 1000)
 * @param maxDelayMs - Maximum delay in milliseconds (default: 10000)
 * @returns Delay in milliseconds
 */
export function calculateBackoffDelay(
  attempt: number,
  baseDelayMs: number = 1000,
  maxDelayMs: number = 10000
): number {
  // Exponential backoff: baseDelay * 2^attempt
  const delay = baseDelayMs * Math.pow(2, attempt)
  return Math.min(delay, maxDelayMs)
}

/**
 * Waits for a specified number of milliseconds.
 *
 * @param ms - Milliseconds to wait
 */
function wait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

/**
 * Retries a function with exponential backoff if it throws a network error.
 *
 * @param fn - The function to retry
 * @param options - Retry options
 * @returns The result of the function
 * @throws The last error if all retries fail
 */
export async function retryOnNetworkError<T>(
  fn: () => Promise<T>,
  options: {
    maxRetries?: number
    baseDelayMs?: number
    maxDelayMs?: number
    onRetry?: (attempt: number, error: Error) => void
  } = {}
): Promise<T> {
  const {
    maxRetries = 1,
    baseDelayMs = 1000,
    maxDelayMs = 10000,
    onRetry,
  } = options

  let lastError: Error | unknown

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      return await fn()
    } catch (error) {
      lastError = error

      // Don't retry if it's not a network error
      if (!isNetworkError(error)) {
        throw error
      }

      // Don't retry on the last attempt
      if (attempt >= maxRetries) {
        throw error
      }

      // Calculate delay and wait before retrying
      const delay = calculateBackoffDelay(attempt, baseDelayMs, maxDelayMs)
      if (onRetry && error instanceof Error) {
        onRetry(attempt + 1, error)
      }
      await wait(delay)
    }
  }

  // This should never be reached, but TypeScript needs it
  throw lastError
}
