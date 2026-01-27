'use server'

import { getTranslations } from 'next-intl/server'
import {
  deleteGame,
  fetchWithAuth,
  getAvailableGames,
  getWaitingLongestGame,
} from '@/lib/api'
import { fetchGameSnapshot } from '@/lib/api/game-room'
import { toErrorResult } from '@/lib/api/action-helpers'
import type { ActionResult, SimpleActionResult } from '@/lib/api/action-helpers'
import type { Game } from '@/lib/types'

export interface CreateGameRequest {
  name?: string
}

/**
 * Server Action to refresh the games list.
 * Automatically refreshes JWT if needed.
 */
export async function refreshGamesListAction(): Promise<ActionResult<Game[]>> {
  try {
    const games = await getAvailableGames()
    return { kind: 'ok', data: games }
  } catch (error) {
    const t = await getTranslations('errors.actions')
    return toErrorResult(error, t('failedToRefreshGamesList'))
  }
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
    const t = await getTranslations('errors.actions')
    return toErrorResult(error, t('failedToCreateGame'))
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
    const t = await getTranslations('errors.actions')
    return toErrorResult(error, t('failedToJoinGame'))
  }
}

export async function spectateGameAction(
  gameId: number
): Promise<ActionResult<Game>> {
  try {
    // Auth is enforced centrally in fetchWithAuth

    const response = await fetchWithAuth(`/api/games/${gameId}/spectate`, {
      method: 'POST',
    })
    const data: { game: Game } = await response.json()
    return { kind: 'ok', data: data.game }
  } catch (error) {
    const t = await getTranslations('errors.actions')
    return toErrorResult(error, t('failedToSpectateGame'))
  }
}

export async function deleteGameAction(
  gameId: number,
  version?: number
): Promise<SimpleActionResult> {
  try {
    // Auth is enforced centrally in fetchWithAuth

    // If no version is provided, fetch the game snapshot to get it
    let finalVersion = version
    if (finalVersion === undefined) {
      try {
        const snapshotResult = await fetchGameSnapshot(gameId)
        if (snapshotResult.kind === 'ok') {
          finalVersion = snapshotResult.msg.version
        } else {
          const t = await getTranslations('errors.actions')
          return toErrorResult(
            new Error('Failed to get lock version from game snapshot'),
            t('failedToDeleteGameNoVersion')
          )
        }
      } catch (error) {
        const t = await getTranslations('errors.actions')
        return toErrorResult(error, t('failedToDeleteGameNoSnapshot'))
      }
    }

    await deleteGame(gameId, finalVersion)
    return { kind: 'ok' }
  } catch (error) {
    const t = await getTranslations('errors.actions')
    return toErrorResult(error, t('failedToDeleteGame'))
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
    const t = await getTranslations('errors.actions')
    return toErrorResult(error, t('failedToFetchGameHistory'))
  }
}

/**
 * Server Action to fetch the game ID that has been waiting the longest.
 * Wraps getWaitingLongestGame to return an ActionResult, ensuring errors are preserved.
 */
export async function getWaitingLongestGameAction(
  excludeGameId?: number
): Promise<ActionResult<number | null>> {
  try {
    const gameId = await getWaitingLongestGame(excludeGameId)
    return { kind: 'ok', data: gameId }
  } catch (error) {
    const t = await getTranslations('errors.actions')
    return toErrorResult(error, t('failedToFetchWaitingGame'))
  }
}
