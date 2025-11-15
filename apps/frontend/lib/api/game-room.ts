'use server'

import { fetchWithAuth, BackendApiError } from '@/lib/api'
import type {
  BidConstraints,
  Card,
  GameSnapshot,
  Seat,
  Trump,
} from '@/lib/game-room/types'
import { isValidSeat } from '@/utils/seat-validation'

export type GameSnapshotResult =
  | {
      kind: 'ok'
      snapshot: GameSnapshot
      etag?: string
      viewerSeat?: Seat | null
      viewerHand: Card[]
      bidConstraints?: BidConstraints | null
    }
  | { kind: 'not_modified' }

export interface SnapshotEnvelope {
  snapshot: GameSnapshot
  viewer_hand?: Card[] | null
  bid_constraints?: {
    zero_bid_locked?: boolean[]
  }
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

      // Validate seat value before using it. Invalid values indicate backend bugs.
      let parsedViewerSeat: Seat | null = null
      if (viewerSeat !== null) {
        if (isValidSeat(viewerSeat)) {
          parsedViewerSeat = viewerSeat
        } else {
          // Log warning for invalid seat values to catch backend bugs
          // Don't clamp - fail hard to surface the issue
          console.warn(
            `Invalid seat value from backend: ${viewerSeat} (expected 0-3, got ${viewerSeat})`
          )
          // Still set to null to indicate invalid seat
          parsedViewerSeat = null
        }
      }
      const viewerHand =
        Array.isArray(body.viewer_hand) &&
        body.viewer_hand.every((card) => typeof card === 'string')
          ? (body.viewer_hand as Card[])
          : []
      const bidConstraints = toBidConstraints(body.bid_constraints) ?? null

      return {
        kind: 'ok',
        snapshot: body.snapshot,
        etag,
        viewerSeat: parsedViewerSeat,
        viewerHand,
        bidConstraints,
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

export async function submitBid(
  gameId: number,
  bid: number,
  etag?: string
): Promise<void> {
  const headers: Record<string, string> = {}
  if (etag) {
    headers['If-Match'] = etag
  }
  await fetchWithAuth(`/api/games/${gameId}/bid`, {
    method: 'POST',
    headers,
    body: JSON.stringify({ bid }),
  })
}

export async function selectTrump(
  gameId: number,
  trump: Trump,
  etag?: string
): Promise<void> {
  const headers: Record<string, string> = {}
  if (etag) {
    headers['If-Match'] = etag
  }
  await fetchWithAuth(`/api/games/${gameId}/trump`, {
    method: 'POST',
    headers,
    body: JSON.stringify({ trump }),
  })
}

export async function submitPlay(
  gameId: number,
  card: string,
  etag?: string
): Promise<void> {
  const headers: Record<string, string> = {}
  if (etag) {
    headers['If-Match'] = etag
  }
  await fetchWithAuth(`/api/games/${gameId}/play`, {
    method: 'POST',
    headers,
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

function toBidConstraints(
  payload?: SnapshotEnvelope['bid_constraints']
): BidConstraints | undefined {
  if (!payload) {
    return undefined
  }

  if (!isZeroBidLockedTuple(payload.zero_bid_locked)) {
    return undefined
  }

  return {
    zeroBidLocked: payload.zero_bid_locked,
  }
}

function isZeroBidLockedTuple(
  value: unknown
): value is [boolean, boolean, boolean, boolean] {
  return (
    Array.isArray(value) &&
    value.length === 4 &&
    value.every((entry) => typeof entry === 'boolean')
  )
}
