'use server'

import { deleteGame, fetchWithAuth, BackendApiError } from '@/lib/api'
import { fetchGameSnapshot } from '@/lib/api/game-room'
import type { Game } from '@/lib/types'

export interface CreateGameRequest {
  name?: string
}

export async function createGameAction(
  request: CreateGameRequest
): Promise<
  { game: Game; error?: never } | { error: BackendApiError; game?: never }
> {
  try {
    // Auth is enforced centrally in fetchWithAuth

    // Frontend ensures a default name is provided if user doesn't enter one
    // Trim the name and send it to backend (backend will use its own default if name is omitted)
    const body: CreateGameRequest = {
      name: request.name?.trim() || undefined,
    }

    const response = await fetchWithAuth('/api/games', {
      method: 'POST',
      body: JSON.stringify(body),
    })
    const data: { game: Game } = await response.json()
    return { game: data.game }
  } catch (error) {
    // Re-throw BackendApiError to preserve traceId
    if (error instanceof BackendApiError) {
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
    // Auth is enforced centrally in fetchWithAuth

    const response = await fetchWithAuth(`/api/games/${gameId}/join`, {
      method: 'POST',
    })
    const data: { game: Game } = await response.json()
    return { game: data.game }
  } catch (error) {
    // Re-throw BackendApiError to preserve traceId
    if (error instanceof BackendApiError) {
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

export type DeleteGameResult =
  | { success: true; error?: never }
  | { error: BackendApiError; success?: never }

export async function deleteGameAction(
  gameId: number,
  etag?: string
): Promise<DeleteGameResult> {
  try {
    // Auth is enforced centrally in fetchWithAuth

    // If no ETag is provided, fetch the game snapshot to get it
    let finalEtag = etag
    if (!finalEtag) {
      try {
        const snapshotResult = await fetchGameSnapshot(gameId)
        if (snapshotResult.kind === 'ok' && snapshotResult.etag) {
          finalEtag = snapshotResult.etag
        }
      } catch (error) {
        // If fetching the snapshot fails, still try to delete
        // The backend will return 428 Precondition Required if ETag is required
        console.warn('Failed to fetch game snapshot for ETag:', error)
      }
    }

    await deleteGame(gameId, finalEtag)
    return { success: true }
  } catch (error) {
    // Re-throw BackendApiError to preserve traceId
    if (error instanceof BackendApiError) {
      return { error }
    }
    // Wrap other errors
    return {
      error: new BackendApiError(
        error instanceof Error ? error.message : 'Failed to delete game',
        500,
        'UNKNOWN_ERROR'
      ),
    }
  }
}
