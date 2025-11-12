// Server-only API functions - must not be imported from client components
// This file uses server-only APIs (cookies, backend JWT resolvers)
'use server'

import {
  BackendJwtMissingError,
  isAuthDisabled,
  requireBackendJwt,
} from '@/lib/server/get-backend-jwt'
import type { Game, GameListResponse, LastActiveGameResponse } from './types'
import { BackendApiError } from './errors'

export interface ProblemDetails {
  type: string
  title: string
  status: number
  detail: string
  code: string
  trace_id: string
  extensions?: unknown
}

// Re-export BackendApiError for convenience (it's also available from ./errors)
export { BackendApiError }

// Removed unused api<T>() helper – fetchWithAuth is the single entrypoint for backend calls

export async function fetchWithAuth(
  endpoint: string,
  options: RequestInit = {}
): Promise<Response> {
  // [AUTH_BYPASS] START - Temporary debugging feature - remove when done
  const disableAuth = isAuthDisabled()

  // Skip auth check if bypass is enabled
  let authHeaders: Record<string, string> = {}
  if (!disableAuth) {
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
  }
  // [AUTH_BYPASS] END

  const baseUrl = process.env.NEXT_PUBLIC_BACKEND_BASE_URL
  if (!baseUrl) {
    throw new Error('NEXT_PUBLIC_BACKEND_BASE_URL not configured')
  }

  const url = `${baseUrl}${endpoint}`
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...authHeaders,
      ...options.headers,
    },
  })

  if (!response.ok) {
    // Try to parse Problem Details error response (RFC 7807)
    const contentType = response.headers.get('content-type')
    const isProblemDetails =
      contentType?.includes('application/problem+json') ||
      contentType?.includes('application/json')

    let errorMessage = response.statusText
    let errorCode: string | undefined
    let traceId = response.headers.get('x-trace-id') || undefined

    if (isProblemDetails) {
      try {
        const problemDetails: ProblemDetails = await response.clone().json()
        errorMessage =
          problemDetails.detail || problemDetails.title || errorMessage
        errorCode = problemDetails.code
        traceId = problemDetails.trace_id || traceId
      } catch {
        // If parsing fails, fall back to status text
      }
    }

    // For 401, ensure we have a proper error code
    if (response.status === 401) {
      errorCode = errorCode || 'UNAUTHORIZED'
      errorMessage = errorMessage || 'Unauthorized'
    }

    throw new BackendApiError(errorMessage, response.status, errorCode, traceId)
  }

  return response
}

// NOTE: /api/private/* endpoints have been removed in the backend. Do not use getMe().

// Game-related API functions

export async function getJoinableGames(): Promise<Game[]> {
  try {
    const response = await fetchWithAuth('/api/games/joinable')
    const data: GameListResponse = await response.json()
    return data.games
  } catch (error) {
    // Handle missing JWT gracefully (user not authenticated)
    if (error instanceof BackendApiError) {
      if (error.status === 401 && error.code === 'MISSING_BACKEND_JWT') {
        console.warn('Skipping joinable games fetch: backend JWT unavailable')
        return []
      }
    } else if (error instanceof TypeError) {
      // Network errors - return empty array to allow page to render
      console.warn('Joinable games fetch failed (network issue)', error)
      return []
    }
    throw error
  }
}

export async function getInProgressGames(): Promise<Game[]> {
  try {
    const response = await fetchWithAuth('/api/games/in-progress')
    const data: GameListResponse = await response.json()
    return data.games
  } catch (error) {
    // Handle missing JWT gracefully (user not authenticated)
    if (error instanceof BackendApiError) {
      if (error.status === 401 && error.code === 'MISSING_BACKEND_JWT') {
        console.warn(
          'Skipping in-progress games fetch: backend JWT unavailable'
        )
        return []
      }
    } else if (error instanceof TypeError) {
      // Network errors - return empty array to allow page to render
      console.warn('In-progress games fetch failed (network issue)', error)
      return []
    }
    throw error
  }
}

export async function getLastActiveGame(): Promise<number | null> {
  try {
    const response = await fetchWithAuth('/api/games/last-active')
    const data: LastActiveGameResponse = await response.json()
    return data.game_id ?? null
  } catch (error) {
    // Handle missing JWT gracefully (user not authenticated)
    if (error instanceof BackendApiError) {
      if (error.status === 401 && error.code === 'MISSING_BACKEND_JWT') {
        console.warn('Skipping last-active fetch: backend JWT unavailable')
        return null
      }
    } else if (error instanceof TypeError) {
      // Network errors - return null to allow page to render
      console.warn('Last active game fetch failed (network issue)', error)
      return null
    }
    throw error
  }
}
