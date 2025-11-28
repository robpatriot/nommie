'use server'

import { deleteGame, fetchWithAuth } from '@/lib/api'
import { fetchGameSnapshot } from '@/lib/api/game-room'
import { toErrorResult } from '@/lib/api/action-helpers'
import type { ActionResult, SimpleActionResult } from '@/lib/api/action-helpers'
import type { Game } from '@/lib/types'

export interface CreateGameRequest {
  name?: string
}

export async function createGameAction(
  request: CreateGameRequest
): Promise<ActionResult<Game>> {
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
    return { kind: 'ok', data: data.game }
  } catch (error) {
    return toErrorResult(error, 'Failed to create game')
  }
}

export async function joinGameAction(
  gameId: number
): Promise<ActionResult<Game>> {
  try {
    // Auth is enforced centrally in fetchWithAuth

    const response = await fetchWithAuth(`/api/games/${gameId}/join`, {
      method: 'POST',
    })
    const data: { game: Game } = await response.json()
    return { kind: 'ok', data: data.game }
  } catch (error) {
    return toErrorResult(error, 'Failed to join game')
  }
}

export async function deleteGameAction(
  gameId: number,
  etag?: string
): Promise<SimpleActionResult> {
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
    return { kind: 'ok' }
  } catch (error) {
    return toErrorResult(error, 'Failed to delete game')
  }
}

export interface GameHistoryApiRound {
  round_no: number
  hand_size: number
  dealer_seat: number
  trump_selector_seat: number | null
  trump: string | null
  bids: [number | null, number | null, number | null, number | null]
  cumulative_scores: [number, number, number, number]
}

export interface GameHistoryApiResponse {
  rounds: GameHistoryApiRound[]
}

export async function getGameHistoryAction(
  gameId: number
): Promise<ActionResult<GameHistoryApiResponse>> {
  try {
    // Auth is enforced centrally in fetchWithAuth
    const response = await fetchWithAuth(`/api/games/${gameId}/history`)
    const data = (await response.json()) as GameHistoryApiResponse
    return { kind: 'ok', data }
  } catch (error) {
    return toErrorResult(error, 'Failed to fetch game history')
  }
}
