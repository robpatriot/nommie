// Server-only utility for retrying fetchWithAuth in Server Components
// This file must never be imported by client code
'use server'

import { fetchWithAuth } from '@/lib/api'
import { isNetworkError } from '@/lib/retry'
import { BackendApiError } from '@/lib/errors'

/**
 * Helper to check if a 5xx error is transient (should be retried).
 * Transient 5xx errors: 502 (Bad Gateway), 503 (Service Unavailable), 504 (Gateway Timeout)
 */
function isTransient5xx(status: number): boolean {
  return status === 502 || status === 503 || status === 504
}

/**
 * Wrapper around fetchWithAuth that retries once for network/transient errors.
 * Only use this in Server Components where a single retry improves UX.
 *
 * Retry policy matches TanStack Query's client-side retry logic:
 * - Retries network errors once
 * - Retries transient 5xx errors (502/503/504) once
 * - Never retries 4xx errors or other 5xx errors
 */
export async function fetchWithAuthWithRetry(
  endpoint: string,
  options: RequestInit = {}
): Promise<Response> {
  try {
    return await fetchWithAuth(endpoint, options)
  } catch (error) {
    // Determine if this error should be retried
    const shouldRetry =
      isNetworkError(error) ||
      (error instanceof BackendApiError && isTransient5xx(error.status))

    if (shouldRetry) {
      // Wait a bit before retry (simple delay, no exponential backoff needed for 1 retry)
      await new Promise((resolve) => setTimeout(resolve, 500))
      return await fetchWithAuth(endpoint, options)
    }

    // Don't retry - re-throw the error
    throw error
  }
}
