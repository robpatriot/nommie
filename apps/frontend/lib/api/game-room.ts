'use server'

import { fetchWithAuth, BackendApiError } from '@/lib/api'
import type { Card, GameSnapshot, Seat, Trump } from '@/lib/game-room/types'

export type GameSnapshotResult =
  | {
      kind: 'ok'
      snapshot: GameSnapshot
      etag?: string
      viewerSeat?: Seat | null
      viewerHand: Card[]
    }
  | { kind: 'not_modified' }

export interface SnapshotEnvelope {
  snapshot: GameSnapshot
  viewer_hand?: Card[] | null
}

export async function fetchGameSnapshot(
  gameId: number,
  options: { etag?: string } = {}
): Promise<GameSnapshotResult> {
  try {
    const response = await fetchWithAuth(`/api/games/${gameId}/snapshot`, {
      headers: options.etag ? { 'If-None-Match': options.etag } : undefined,
    })

    const body = (await response.json()) as SnapshotEnvelope
    const etag = response.headers.get('etag') ?? undefined
    const viewerSeatHeader = response.headers.get('x-viewer-seat')
    const viewerSeat =
      viewerSeatHeader !== null && viewerSeatHeader.trim() !== ''
        ? Number.parseInt(viewerSeatHeader, 10)
        : null

    // Validate and clamp seat value to 0-3 range. Log warning if out of range to catch backend bugs.
    let parsedViewerSeat: Seat | null = null
    if (viewerSeat !== null && Number.isFinite(viewerSeat)) {
      const originalValue = viewerSeat
      const clampedValue = Math.max(0, Math.min(3, viewerSeat))
      if (originalValue !== clampedValue) {
        console.warn(
          `Seat value out of range: ${originalValue} (clamped to ${clampedValue})`
        )
      }
      parsedViewerSeat = clampedValue as Seat
    }
    const viewerHand =
      Array.isArray(body.viewer_hand) &&
      body.viewer_hand.every((card) => typeof card === 'string')
        ? (body.viewer_hand as Card[])
        : []

    return {
      kind: 'ok',
      snapshot: body.snapshot,
      etag,
      viewerSeat: parsedViewerSeat,
      viewerHand,
    }
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

export async function selectTrump(gameId: number, trump: Trump): Promise<void> {
  await fetchWithAuth(`/api/games/${gameId}/trump`, {
    method: 'POST',
    body: JSON.stringify({ trump }),
  })
}

export async function submitPlay(gameId: number, card: string): Promise<void> {
  await fetchWithAuth(`/api/games/${gameId}/play`, {
    method: 'POST',
    body: JSON.stringify({ card }),
  })
}

export interface ManageAiSeatPayload {
  seat?: number
  registryName?: string
  registryVersion?: string
  seed?: number
}

function buildAiSeatBody(payload?: ManageAiSeatPayload) {
  const body: Record<string, unknown> = {}
  if (!payload) {
    return body
  }

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

  return body
}

export async function addAiSeat(
  gameId: number,
  payload?: ManageAiSeatPayload
): Promise<void> {
  await fetchWithAuth(`/api/games/${gameId}/ai/add`, {
    method: 'POST',
    body: JSON.stringify(buildAiSeatBody(payload)),
  })
}

export async function updateAiSeat(
  gameId: number,
  payload: ManageAiSeatPayload
): Promise<void> {
  await fetchWithAuth(`/api/games/${gameId}/ai/update`, {
    method: 'POST',
    body: JSON.stringify(buildAiSeatBody(payload)),
  })
}

export async function removeAiSeat(
  gameId: number,
  payload?: ManageAiSeatPayload
): Promise<void> {
  await fetchWithAuth(`/api/games/${gameId}/ai/remove`, {
    method: 'POST',
    body: JSON.stringify(buildAiSeatBody(payload)),
  })
}

export interface AiRegistryEntry {
  name: string
  version: string
}

export async function listRegisteredAis(): Promise<AiRegistryEntry[]> {
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
  return Array.isArray(data.ais) ? data.ais : []
}
