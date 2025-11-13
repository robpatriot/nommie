// Server-only API functions - must not be imported from client components
// This file uses server-only APIs (cookies, backend JWT resolvers)
'use server'

import {
  BackendJwtMissingError,
  requireBackendJwt,
} from '@/lib/server/get-backend-jwt'
import type { Game, GameListResponse, LastActiveGameResponse } from './types'
import { BackendApiError } from './errors'
import { retryOnNetworkError } from './retry'
import { parseErrorResponse } from './api/error-parsing'

// Re-export BackendApiError for convenience (it's also available from ./errors)
export { BackendApiError }

// Re-export ProblemDetails interface for backward compatibility
export type { ProblemDetails } from './api/error-parsing'

// Removed unused api<T>() helper – fetchWithAuth is the single entrypoint for backend calls

export async function fetchWithAuth(
  endpoint: string,
  options: RequestInit = {}
): Promise<Response> {
  // Always require backend JWT for authenticated requests
  let authHeaders: Record<string, string> = {}
  try {
    const backendJwt = await requireBackendJwt()

    authHeaders = {
      Authorization: `Bearer ${backendJwt}`,
    }
  } catch (error) {
    if (error instanceof BackendJwtMissingError) {
      throw new BackendApiError(
        'Authentication required',
        401,
        'MISSING_BACKEND_JWT'
      )
    }
    throw error
  }

  const baseUrl = process.env.NEXT_PUBLIC_BACKEND_BASE_URL
  if (!baseUrl) {
    throw new Error('NEXT_PUBLIC_BACKEND_BASE_URL not configured')
  }

  const url = `${baseUrl}${endpoint}`

  // Retry network errors with exponential backoff (1 retry by default)
  // Application errors (4xx, 5xx) are not retried
  const response = await retryOnNetworkError(
    async () => {
      return await fetch(url, {
        ...options,
        headers: {
          'Content-Type': 'application/json',
          ...authHeaders,
          ...options.headers,
        },
      })
    },
    {
      maxRetries: 1,
      baseDelayMs: 500,
      maxDelayMs: 2000,
    }
  )

  if (!response.ok) {
    // Parse Problem Details error response (RFC 7807)
    const parsedError = await parseErrorResponse(response)
    throw new BackendApiError(
      parsedError.message,
      response.status,
      parsedError.code,
      parsedError.traceId
    )
  }

  return response
}

// NOTE: /api/private/* endpoints have been removed in the backend. Do not use getMe().

// Game-related API functions

export async function getJoinableGames(): Promise<Game[]> {
  const response = await fetchWithAuth('/api/games/joinable')
  const data: GameListResponse = await response.json()
  return data.games
}

export async function getInProgressGames(): Promise<Game[]> {
  const response = await fetchWithAuth('/api/games/in-progress')
  const data: GameListResponse = await response.json()
  return data.games
}

export async function getLastActiveGame(): Promise<number | null> {
  const response = await fetchWithAuth('/api/games/last-active')
  const data: LastActiveGameResponse = await response.json()
  return data.game_id ?? null
}

export async function deleteGame(gameId: number, etag?: string): Promise<void> {
  const headers: Record<string, string> = {}
  if (etag) {
    headers['If-Match'] = etag
  }
  await fetchWithAuth(`/api/games/${gameId}`, {
    method: 'DELETE',
    headers,
  })
}
