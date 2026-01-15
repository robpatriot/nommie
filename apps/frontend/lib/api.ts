// Server-only API functions - must not be imported from client components
// This file uses server-only APIs (cookies, backend JWT resolvers)
'use server'

import { getTranslations } from 'next-intl/server'

import {
  getBackendJwtReadOnly,
  BackendJwtError,
} from '@/lib/auth/refresh-backend-jwt'
import { getBackendBaseUrlOrThrow } from '@/auth'
import { errorCodeToMessageKey } from '@/i18n/errors'
import type { Game, GameListResponse, LastActiveGameResponse } from './types'
import { BackendApiError } from './errors'
import { parseErrorResponse } from './api/error-parsing'
import {
  markBackendUp,
  shouldLogError,
  isInStartupWindow,
} from '@/lib/server/backend-status'
import { isBackendConnectionError } from '@/lib/server/connection-errors'
import { fetchWithAuthWithRetry } from './server/fetch-with-retry'
import { logError, logWarning } from '@/lib/logging/error-logger'

// Re-export BackendApiError for convenience (it's also available from ./errors)
export { BackendApiError }

async function getLocalizedErrorMessageForCode(
  code: string,
  fallback: string
): Promise<string> {
  try {
    const t = await getTranslations('errors')
    const key = errorCodeToMessageKey(code).replace('errors.', '')
    return t(key)
  } catch {
    return fallback
  }
}

/**
 * Make an authenticated API request to the backend.
 * Works from both Server Components and Server Actions.
 * Reads JWT from cookie (does not refresh - middleware handles refresh).
 */
export async function fetchWithAuth(
  endpoint: string,
  options: RequestInit = {}
): Promise<Response> {
  // Get backend JWT from cookie (read-only - middleware handles refresh)
  let authHeaders: Record<string, string> = {}
  try {
    const backendJwt = await getBackendJwtReadOnly()
    if (!backendJwt) {
      // No JWT available - middleware should have refreshed it, but if not,
      // this is an auth error
      throw new BackendJwtError('Authentication required')
    }
    authHeaders = {
      Authorization: `Bearer ${backendJwt}`,
    }
  } catch (error) {
    if (error instanceof BackendJwtError) {
      // If the backend explicitly reports that this email is not allowed,
      // surface a stable 403 EMAIL_NOT_ALLOWED signal to callers.
      if (error.message === 'EMAIL_NOT_ALLOWED') {
        const message = await getLocalizedErrorMessageForCode(
          'EMAIL_NOT_ALLOWED',
          'Access restricted. Please contact support if you believe this is an error.'
        )
        throw new BackendApiError(message, 403, 'EMAIL_NOT_ALLOWED')
      }
      // During startup, if backend isn't ready, use 503 (Service Unavailable)
      // Otherwise, use 401 (Unauthorized) for actual auth issues
      if (isInStartupWindow() && error.message.includes('starting up')) {
        const message = await getLocalizedErrorMessageForCode(
          'BACKEND_STARTING',
          'Backend is starting up, please try again shortly'
        )
        throw new BackendApiError(message, 503, 'BACKEND_STARTING')
      }
      const message = await getLocalizedErrorMessageForCode(
        'MISSING_BACKEND_JWT',
        'Authentication required'
      )
      throw new BackendApiError(message, 401, 'MISSING_BACKEND_JWT')
    }
    throw error
  }

  const baseUrl = getBackendBaseUrlOrThrow()
  const url = `${baseUrl}${endpoint}`

  let response: Response
  try {
    // Direct fetch - retry logic is handled by TanStack Query for client-side requests
    response = await fetch(url, {
      ...options,
      headers: {
        'Content-Type': 'application/json',
        ...authHeaders,
        ...options.headers,
      },
    })

    // Mark backend as up if we got a response (even if error) - connection succeeded
    // This helps differentiate startup failures from runtime failures
    markBackendUp()
  } catch (error) {
    // Handle network errors - check if we should log
    const isConnectionError = isBackendConnectionError(error)

    // Only log connection errors if we should (outside startup window or runtime failure)
    if (shouldLogError() && isConnectionError) {
      logError('Backend connection error during API request', error, {
        endpoint,
      })
    } else if (shouldLogError() && !isConnectionError) {
      // Other errors should be logged if outside startup window
      logError('Network error during API request', error, { endpoint })
    }

    throw error
  }

  if (!response.ok) {
    // Handle rate limit errors (429) - these are application errors, not network errors
    if (response.status === 429) {
      // Rate limit exceeded - parse error response but don't log during startup
      const parsedError = await parseErrorResponse(response)
      if (shouldLogError()) {
        logWarning('Rate limit exceeded', {
          code: parsedError.code,
          message: parsedError.message,
          endpoint,
        })
      }
      throw new BackendApiError(
        parsedError.message,
        response.status,
        parsedError.code,
        parsedError.traceId
      )
    }

    // Parse Problem Details error response (RFC 7807)
    const parsedError = await parseErrorResponse(response)
    // Only log if we should (outside startup window or runtime failure)
    if (shouldLogError() && response.status >= 500) {
      // Server errors should be logged
      logError('Backend API error', new Error(parsedError.message), {
        status: response.status,
        code: parsedError.code,
        traceId: parsedError.traceId,
        endpoint,
      })
    }
    throw new BackendApiError(
      parsedError.message,
      response.status,
      parsedError.code,
      parsedError.traceId
    )
  }

  return response
}

// Game-related API functions

/**
 * Fetch available games list.
 * Uses fetchWithAuthWithRetry for improved SSR resilience on initial page load.
 */
export async function getAvailableGames(): Promise<Game[]> {
  const response = await fetchWithAuthWithRetry('/api/games/overview')
  const data: GameListResponse = await response.json()
  return data.games
}

export async function getLastActiveGame(): Promise<number | null> {
  const response = await fetchWithAuth('/api/games/last-active')
  const data: LastActiveGameResponse = await response.json()
  return data.game_id ?? null
}

export async function deleteGame(
  gameId: number,
  version: number
): Promise<void> {
  await fetchWithAuth(`/api/games/${gameId}`, {
    method: 'DELETE',
    body: JSON.stringify({ version: version }),
  })
}
