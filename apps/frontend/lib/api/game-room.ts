'use server'

import { fetchWithAuth, BackendApiError } from '@/lib/api'
import type { GameSnapshot } from '@/lib/game-room/types'

export type GameSnapshotResult =
  | { kind: 'ok'; snapshot: GameSnapshot; etag?: string }
  | { kind: 'not_modified' }

export async function fetchGameSnapshot(
  gameId: number,
  options: { etag?: string } = {}
): Promise<GameSnapshotResult> {
  try {
    const response = await fetchWithAuth(`/api/games/${gameId}/snapshot`, {
      headers: options.etag ? { 'If-None-Match': options.etag } : undefined,
    })
    const snapshot: GameSnapshot = await response.json()
    const etag = response.headers.get('etag') ?? undefined
    return { kind: 'ok', snapshot, etag }
  } catch (error) {
    if (error instanceof BackendApiError && error.status === 304) {
      return { kind: 'not_modified' }
    }
    throw error
  }
}

export async function fetchSeatDisplayNames(
  gameId: number
): Promise<[string, string, string, string]> {
  const seats = (await Promise.all(
    [0, 1, 2, 3].map(async (seat) => {
      try {
        const response = await fetchWithAuth(
          `/api/games/${gameId}/players/${seat}/display_name`
        )
        const data: { display_name?: unknown } = await response.json()
        if (typeof data.display_name === 'string' && data.display_name.trim()) {
          return data.display_name.trim()
        }
      } catch (error) {
        if (error instanceof BackendApiError && error.status === 404) {
          // Endpoint may not exist yet – fall back to placeholder name
          return `Player ${seat + 1}`
        }
        console.warn('Failed to load display name', { seat, error })
      }
      return `Player ${seat + 1}`
    })
  )) as [string, string, string, string]

  return seats
}
