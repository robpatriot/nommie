'use server'

import { auth } from '@/auth'
import { fetchWithAuth, BackendApiError } from '@/lib/api'
import type { Game } from '@/lib/types'

export interface CreateGameRequest {
  name?: string
  starting_dealer_pos?: number | null
}

export async function createGameAction(
  request: CreateGameRequest
): Promise<
  { game: Game; error?: never } | { error: BackendApiError; game?: never }
> {
  try {
    // Verify auth
    const session = await auth()
    if (!session?.backendJwt) {
      return {
        error: new BackendApiError('Not authenticated', 401, 'UNAUTHORIZED'),
      }
    }

    const response = await fetchWithAuth('/api/games', {
      method: 'POST',
      body: JSON.stringify(request),
    })
    const data: { game: Game } = await response.json()
    return { game: data.game }
  } catch (error) {
    // Re-throw BackendApiError to preserve traceId
    if (error instanceof BackendApiError) {
      // Provide a more user-friendly message for 404s (endpoint not implemented)
      if (error.status === 404) {
        return {
          error: new BackendApiError(
            'Create game endpoint not yet implemented on the backend',
            error.status,
            error.code,
            error.traceId
          ),
        }
      }
      return { error }
    }
    // Wrap other errors
    return {
      error: new BackendApiError(
        error instanceof Error ? error.message : 'Failed to create game',
        500,
        'UNKNOWN_ERROR'
      ),
    }
  }
}

export async function joinGameAction(
  gameId: number
): Promise<
  { game: Game; error?: never } | { error: BackendApiError; game?: never }
> {
  try {
    // Verify auth
    const session = await auth()
    if (!session?.backendJwt) {
      return {
        error: new BackendApiError('Not authenticated', 401, 'UNAUTHORIZED'),
      }
    }

    const response = await fetchWithAuth(`/api/games/${gameId}/join`, {
      method: 'POST',
    })
    const data: { game: Game } = await response.json()
    return { game: data.game }
  } catch (error) {
    // Re-throw BackendApiError to preserve traceId
    if (error instanceof BackendApiError) {
      // Provide a more user-friendly message for 404s (endpoint not implemented)
      if (error.status === 404) {
        return {
          error: new BackendApiError(
            'Join game endpoint not yet implemented on the backend',
            error.status,
            error.code,
            error.traceId
          ),
        }
      }
      return { error }
    }
    // Wrap other errors
    return {
      error: new BackendApiError(
        error instanceof Error ? error.message : 'Failed to join game',
        500,
        'UNKNOWN_ERROR'
      ),
    }
  }
}
