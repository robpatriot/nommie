// Server-only API functions - must not be imported from client components
// This file uses server-only APIs (cookies, backend session resolver)
'use server'

import { getTranslations } from 'next-intl/server'

import { getBackendSessionCookie } from '@/lib/auth/backend-jwt-cookie.server'
import { getBackendBaseUrlOrThrow } from '@/auth'
import { errorCodeToMessageKey } from '@/i18n/errors'
import type { Game, GameListResponse, LastActiveGameResponse } from './types'
import { BackendApiError } from './errors'
import { parseErrorResponse } from './api/error-parsing'
import {
  markBackendUp,
  markBackendDown,
  shouldLogError,
  isBackendReady,
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
 * Reads session token from cookie and forwards it as a Cookie header.
 */
export async function fetchWithAuth(
  endpoint: string,
  options: RequestInit = {}
): Promise<Response> {
  const token = await getBackendSessionCookie()
  if (!token) {
    const message = await getLocalizedErrorMessageForCode(
      'UNAUTHORIZED',
      'Authentication required'
    )
    throw new BackendApiError(message, 401, 'UNAUTHORIZED')
  }

  const authHeaders: Record<string, string> = {
    Cookie: `backend_session=${token}`,
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

    // Only mark backend up when we got a successful response. 503/5xx means not ready.
    if (response.ok) {
      markBackendUp('fetchWithAuth')
    }
  } catch (error) {
    const isConnectionError = isBackendConnectionError(error)

    if (isConnectionError) {
      const msg = error instanceof Error ? error.message : String(error)
      markBackendDown(msg, 'fetchWithAuth_connection')
    }

    if (isBackendReady() && shouldLogError()) {
      if (isConnectionError) {
        logError('Backend connection error during API request', error, {
          endpoint,
        })
      } else {
        logError('Network error during API request', error, { endpoint })
      }
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

    // Mark backend as down when it returns 503 so server-side readiness is correct
    if (response.status === 503) {
      markBackendDown(parsedError.message, 'fetchWithAuth_503')
    }

    // Only log 5xx as errors when we should; skip 503 (expected when backend is not ready)
    const isServiceUnavailable =
      response.status === 503 || parsedError.code === 'SERVICE_UNAVAILABLE'
    if (shouldLogError() && response.status >= 500 && !isServiceUnavailable) {
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

export async function getWaitingLongestGame(): Promise<number[]> {
  const response = await fetchWithAuth('/api/games/waiting-longest')
  const data: LastActiveGameResponse = await response.json()
  return data.game_ids
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
