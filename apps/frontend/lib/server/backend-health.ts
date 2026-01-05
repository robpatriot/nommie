// Server-only utility to check backend health
// This file must never be imported by client code

import { getBackendBaseUrlOrThrow } from '@/auth'
import { markBackendUp, shouldLogError } from './backend-status'
import { isBackendConnectionError } from './connection-errors'
import { logError } from '@/lib/logging/error-logger'

/**
 * Checks if the backend is available by hitting the /health endpoint.
 * This endpoint is not rate-limited, making it safe for startup probing.
 *
 * @returns true if backend is healthy, false otherwise
 */
export async function checkBackendHealth(): Promise<boolean> {
  try {
    const backendBase = getBackendBaseUrlOrThrow()
    const response = await fetch(`${backendBase}/health`, {
      method: 'GET',
      headers: { 'Content-Type': 'application/json' },
      // Use a shorter timeout for health checks
      signal: AbortSignal.timeout(5000), // 5 second timeout
    })

    if (response.ok) {
      // Backend is up - mark it as such
      markBackendUp()
      return true
    }

    return false
  } catch (error) {
    // Check if this is a connection error
    const isConnectionError = isBackendConnectionError(error)

    // Only log if we should (outside startup window or runtime failure)
    if (shouldLogError() && isConnectionError) {
      logError('Backend health check failed (connection error)', error)
    } else if (shouldLogError() && !isConnectionError) {
      // Non-connection errors should be logged if outside startup window
      logError('Backend health check failed', error)
    }

    return false
  }
}
