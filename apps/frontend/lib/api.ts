// Server-only API functions - must not be imported from client components
// This file uses server-only APIs (cookies, backend JWT resolvers)
'use server'

import { isAuthDisabled, requireBackendJwt } from '@/lib/server/get-backend-jwt'
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
    const backendJwt = await requireBackendJwt()

    authHeaders = {
      Authorization: `Bearer ${backendJwt}`,
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
    // TODO: Remove once backend endpoint is implemented
    // If endpoint doesn't exist yet, return empty array
    // In production, you'd want to handle this differently
    if (error instanceof BackendApiError && error.status === 404) {
      console.warn('Joinable games endpoint not yet implemented')
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
    // TODO: Remove once backend endpoint is implemented
    // If endpoint doesn't exist yet, return empty array
    if (error instanceof BackendApiError && error.status === 404) {
      console.warn('In-progress games endpoint not yet implemented')
      return []
    }
    throw error
  }
}

export async function getLastActiveGame(): Promise<number | null> {
  try {
    const response = await fetchWithAuth('/api/games/last-active')
    const data: LastActiveGameResponse = await response.json()
    return data.game_id
  } catch (error) {
    // TODO: Remove once backend endpoint is implemented
    // If endpoint doesn't exist yet, return null
    if (error instanceof BackendApiError && error.status === 404) {
      console.warn('Last active game endpoint not yet implemented')
      return null
    }
    throw error
  }
}
