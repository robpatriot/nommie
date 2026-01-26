'use server'

import { fetchWithAuth, BackendApiError } from '@/lib/api'
import { fetchWithAuthWithRetry } from '@/lib/server/fetch-with-retry'
import type { Card, GameSnapshot, Trump } from '@/lib/game-room/types'
import { isGameStateMsg, isWireMsg } from '@/lib/game-room/protocol/types'
import type { GameStateMsg } from '@/lib/game-room/protocol/types'

export type GameSnapshotResult =
  | {
      kind: 'ok'
      msg: GameStateMsg
      etag?: string
    }
  | { kind: 'not_modified' }

export interface SnapshotEnvelope {
  snapshot: GameSnapshot
  version?: number
  viewer_hand?: Card[] | null
  bid_constraints?: {
    zero_bid_locked?: boolean[]
  }
}

/**
 * Fetch game snapshot.
 * Works from both Server Components and Server Actions.
 * Uses fetchWithAuthWithRetry for improved SSR resilience on initial page load.
 * Automatically refreshes JWT if needed.
 */
export async function fetchGameSnapshot(
  gameId: number,
  options: { etag?: string } = {}
): Promise<GameSnapshotResult> {
  try {
    const response = await fetchWithAuthWithRetry(
      `/api/games/${gameId}/snapshot`,
      {
        headers: options.etag ? { 'If-None-Match': options.etag } : undefined,
      }
    )

    const raw = (await response.json()) as unknown
    const etag = response.headers.get('etag') ?? undefined

    if (!isWireMsg(raw)) {
      throw new Error('Invalid snapshot response: not a wire message')
    }
    if (!isGameStateMsg(raw)) {
      throw new Error(
        `Invalid snapshot response: expected game_state, got ${String(
          (raw as { type?: unknown }).type
        )}`
      )
    }

    return { kind: 'ok', msg: raw, etag }
  } catch (error) {
    if (error instanceof BackendApiError && error.status === 304) {
      return { kind: 'not_modified' }
    }
    throw error
  }
}

export async function markPlayerReady(
  gameId: number,
  isReady: boolean,
  version: number
): Promise<void> {
  await fetchWithAuth(`/api/games/${gameId}/ready`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ is_ready: isReady, version: version }),
  })
}

export async function leaveGame(
  gameId: number,
  version: number
): Promise<void> {
  const response = await fetchWithAuth(`/api/games/${gameId}/leave`, {
    method: 'DELETE',
    body: JSON.stringify({ version: version }),
  })
  // For 204 No Content responses, we don't need to parse JSON
  // Just verify the response was successful (fetchWithAuth throws on error)
  if (!response.ok) {
    throw new Error(`Leave game failed with status ${response.status}`)
  }
}

export async function rejoinGame(
  gameId: number,
  version: number
): Promise<{ version: number }> {
  const response = await fetchWithAuth(`/api/games/${gameId}/rejoin`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ version: version }),
  })

  if (!response.ok) {
    throw new Error(`Rejoin game failed with status ${response.status}`)
  }

  const data = (await response.json()) as { version: number }
  return data
}

export async function submitBid(
  gameId: number,
  bid: number,
  version: number
): Promise<void> {
  await fetchWithAuth(`/api/games/${gameId}/bid`, {
    method: 'POST',
    body: JSON.stringify({ bid, version: version }),
  })
}

export async function selectTrump(
  gameId: number,
  trump: Trump,
  version: number
): Promise<void> {
  await fetchWithAuth(`/api/games/${gameId}/trump`, {
    method: 'POST',
    body: JSON.stringify({ trump, version: version }),
  })
}

export async function submitPlay(
  gameId: number,
  card: string,
  version: number
): Promise<void> {
  await fetchWithAuth(`/api/games/${gameId}/play`, {
    method: 'POST',
    body: JSON.stringify({ card, version: version }),
  })
}

export interface ManageAiSeatPayload {
  seat?: number
  registryName?: string
  registryVersion?: string
  seed?: number
}

function buildAiSeatBody(
  payload: ManageAiSeatPayload | undefined,
  version: number
) {
  const body: Record<string, unknown> = {
    version: version,
  }

  if (payload) {
    if (payload.seat !== undefined) {
      body.seat = payload.seat
    }
    if (payload.registryName) {
      body.registry_name = payload.registryName
    }
    if (payload.registryVersion) {
      body.registry_version = payload.registryVersion
    }
    if (typeof payload.seed === 'number') {
      body.seed = payload.seed
    }
  }

  return body
}

export async function addAiSeat(
  gameId: number,
  version: number,
  payload?: ManageAiSeatPayload
): Promise<void> {
  await fetchWithAuth(`/api/games/${gameId}/ai/add`, {
    method: 'POST',
    body: JSON.stringify(buildAiSeatBody(payload, version)),
  })
}

export async function updateAiSeat(
  gameId: number,
  version: number,
  payload: ManageAiSeatPayload
): Promise<void> {
  await fetchWithAuth(`/api/games/${gameId}/ai/update`, {
    method: 'POST',
    body: JSON.stringify(buildAiSeatBody(payload, version)),
  })
}

export async function removeAiSeat(
  gameId: number,
  version: number,
  payload?: ManageAiSeatPayload
): Promise<void> {
  await fetchWithAuth(`/api/games/${gameId}/ai/remove`, {
    method: 'POST',
    body: JSON.stringify(buildAiSeatBody(payload, version)),
  })
}

export interface AiRegistryEntry {
  name: string
  version: string
}

export interface AiRegistryResponse {
  entries: AiRegistryEntry[]
  defaultName: string
}

export async function listRegisteredAis(): Promise<AiRegistryResponse> {
  const response = await fetchWithAuth('/api/games/ai/registry', {
    method: 'GET',
  })

  if (!response.ok) {
    throw new BackendApiError(
      'Failed to load AI registry',
      response.status,
      'AI_REGISTRY_ERROR'
    )
  }

  const data = await response.json()
  return {
    entries: Array.isArray(data.ais) ? data.ais : [],
    defaultName: data.default_name ?? 'Tactician',
  }
}
