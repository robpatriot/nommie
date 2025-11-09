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

export async function markPlayerReady(gameId: number): Promise<void> {
  await fetchWithAuth(`/api/games/${gameId}/ready`, {
    method: 'POST',
  })
}

export async function submitBid(gameId: number, bid: number): Promise<void> {
  await fetchWithAuth(`/api/games/${gameId}/bid`, {
    method: 'POST',
    body: JSON.stringify({ bid }),
  })
}

export async function submitPlay(gameId: number, card: string): Promise<void> {
  await fetchWithAuth(`/api/games/${gameId}/play`, {
    method: 'POST',
    body: JSON.stringify({ card }),
  })
}

export async function addAiSeat(gameId: number, seat?: number): Promise<void> {
  await fetchWithAuth(`/api/games/${gameId}/ai/add`, {
    method: 'POST',
    body: seat === undefined ? undefined : JSON.stringify({ seat }),
  })
}

export async function removeAiSeat(
  gameId: number,
  seat?: number
): Promise<void> {
  await fetchWithAuth(`/api/games/${gameId}/ai/remove`, {
    method: 'POST',
    body: seat === undefined ? undefined : JSON.stringify({ seat }),
  })
}
