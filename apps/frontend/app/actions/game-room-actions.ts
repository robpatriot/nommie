'use server'

import { BackendApiError } from '@/lib/api'
import {
  fetchGameSnapshot,
  markPlayerReady,
  submitBid,
  selectTrump,
  submitPlay,
  addAiSeat,
  updateAiSeat,
  removeAiSeat,
  listRegisteredAis,
  type AiRegistryEntry,
} from '@/lib/api/game-room'
import { DEFAULT_VIEWER_SEAT } from '@/lib/game-room/constants'
import type { Card, GameSnapshot, Seat, Trump } from '@/lib/game-room/types'

export interface GameRoomSnapshotRequest {
  gameId: number
  etag?: string
}

export type GameRoomSnapshotActionResult =
  | { kind: 'ok'; data: GameRoomSnapshotPayload }
  | { kind: 'not_modified' }
  | { kind: 'error'; message: string; status: number; traceId?: string }

export interface GameRoomSnapshotPayload {
  snapshot: GameSnapshot
  etag?: string
  playerNames: [string, string, string, string]
  viewerSeat: Seat | null
  viewerHand: Card[]
  timestamp: string
  hostSeat: Seat
}

export async function getGameRoomSnapshotAction(
  request: GameRoomSnapshotRequest
): Promise<GameRoomSnapshotActionResult> {
  try {
    const snapshotResult = await fetchGameSnapshot(request.gameId, {
      etag: request.etag,
    })

    if (snapshotResult.kind === 'not_modified') {
      return { kind: 'not_modified' }
    }

    const seating = snapshotResult.snapshot.game.seating
    const playerNames = seating.map((seat, index) => {
      const name = seat.display_name?.trim()
      if (name && name.length > 0) {
        return name
      }
      return `Seat ${index + 1}`
    }) as [string, string, string, string]

    const hostSeat = (snapshotResult.snapshot.game.host_seat ??
      DEFAULT_VIEWER_SEAT) as Seat
    const viewerSeat: Seat | null =
      typeof snapshotResult.viewerSeat === 'number'
        ? (snapshotResult.viewerSeat as Seat)
        : null

    return {
      kind: 'ok',
      data: {
        snapshot: snapshotResult.snapshot,
        etag: snapshotResult.etag,
        playerNames,
        viewerSeat,
        viewerHand: snapshotResult.viewerHand ?? [],
        timestamp: new Date().toISOString(),
        hostSeat,
      },
    }
  } catch (error) {
    if (error instanceof BackendApiError) {
      return {
        kind: 'error',
        message: error.message,
        status: error.status,
        traceId: error.traceId,
      }
    }

    throw error
  }
}

export type SimpleActionResult =
  | { kind: 'ok' }
  | { kind: 'error'; message: string; status: number; traceId?: string }

export async function markPlayerReadyAction(
  gameId: number
): Promise<SimpleActionResult> {
  try {
    await markPlayerReady(gameId)
    return { kind: 'ok' }
  } catch (error) {
    if (error instanceof BackendApiError) {
      return {
        kind: 'error',
        message: error.message,
        status: error.status,
        traceId: error.traceId,
      }
    }

    throw error
  }
}

export interface SubmitBidRequest {
  gameId: number
  bid: number
}

export async function submitBidAction(
  request: SubmitBidRequest
): Promise<SimpleActionResult> {
  const bidValue = Math.trunc(request.bid)

  if (!Number.isFinite(bidValue) || bidValue < 0) {
    return {
      kind: 'error',
      message: 'Invalid bid value',
      status: 400,
    }
  }

  try {
    await submitBid(request.gameId, bidValue)
    return { kind: 'ok' }
  } catch (error) {
    if (error instanceof BackendApiError) {
      return {
        kind: 'error',
        message: error.message,
        status: error.status,
        traceId: error.traceId,
      }
    }

    throw error
  }
}

export interface SelectTrumpRequest {
  gameId: number
  trump: Trump
}

export async function selectTrumpAction(
  request: SelectTrumpRequest
): Promise<SimpleActionResult> {
  try {
    await selectTrump(request.gameId, request.trump)
    return { kind: 'ok' }
  } catch (error) {
    if (error instanceof BackendApiError) {
      return {
        kind: 'error',
        message: error.message,
        status: error.status,
        traceId: error.traceId,
      }
    }

    throw error
  }
}

export interface SubmitPlayRequest {
  gameId: number
  card: string
}

export async function submitPlayAction(
  request: SubmitPlayRequest
): Promise<SimpleActionResult> {
  const card = request.card.trim()

  if (!card) {
    return {
      kind: 'error',
      message: 'Card selection required',
      status: 400,
    }
  }

  try {
    await submitPlay(request.gameId, card)
    return { kind: 'ok' }
  } catch (error) {
    if (error instanceof BackendApiError) {
      return {
        kind: 'error',
        message: error.message,
        status: error.status,
        traceId: error.traceId,
      }
    }

    throw error
  }
}

export interface ManageAiSeatRequest {
  gameId: number
  seat?: Seat
  registryName?: string
  registryVersion?: string
  seed?: number
}

export async function addAiSeatAction(
  request: ManageAiSeatRequest
): Promise<SimpleActionResult> {
  if (
    request.seat !== undefined &&
    (request.seat < 0 || request.seat > 3 || Number.isNaN(request.seat))
  ) {
    return {
      kind: 'error',
      message: 'Seat must be between 0 and 3',
      status: 400,
    }
  }

  try {
    await addAiSeat(request.gameId, {
      seat: request.seat,
      registryName: request.registryName,
      registryVersion: request.registryVersion,
      seed: request.seed,
    })
    return { kind: 'ok' }
  } catch (error) {
    if (error instanceof BackendApiError) {
      return {
        kind: 'error',
        message: error.message,
        status: error.status,
        traceId: error.traceId,
      }
    }

    throw error
  }
}

export async function removeAiSeatAction(
  request: ManageAiSeatRequest
): Promise<SimpleActionResult> {
  if (
    request.seat !== undefined &&
    (request.seat < 0 || request.seat > 3 || Number.isNaN(request.seat))
  ) {
    return {
      kind: 'error',
      message: 'Seat must be between 0 and 3',
      status: 400,
    }
  }

  try {
    await removeAiSeat(request.gameId, {
      seat: request.seat,
    })
    return { kind: 'ok' }
  } catch (error) {
    if (error instanceof BackendApiError) {
      return {
        kind: 'error',
        message: error.message,
        status: error.status,
        traceId: error.traceId,
      }
    }

    throw error
  }
}

export async function updateAiSeatAction(
  request: ManageAiSeatRequest
): Promise<SimpleActionResult> {
  if (
    request.seat === undefined ||
    Number.isNaN(request.seat) ||
    request.seat < 0 ||
    request.seat > 3
  ) {
    return {
      kind: 'error',
      message: 'Seat must be provided between 0 and 3',
      status: 400,
    }
  }

  try {
    await updateAiSeat(request.gameId, {
      seat: request.seat,
      registryName: request.registryName,
      registryVersion: request.registryVersion,
      seed: request.seed,
    })
    return { kind: 'ok' }
  } catch (error) {
    if (error instanceof BackendApiError) {
      return {
        kind: 'error',
        message: error.message,
        status: error.status,
        traceId: error.traceId,
      }
    }

    throw error
  }
}

export type AiRegistryActionResult =
  | { kind: 'ok'; ais: AiRegistryEntry[] }
  | { kind: 'error'; message: string; status: number; traceId?: string }

export async function fetchAiRegistryAction(): Promise<AiRegistryActionResult> {
  try {
    const ais = await listRegisteredAis()
    return { kind: 'ok', ais }
  } catch (error) {
    if (error instanceof BackendApiError) {
      return {
        kind: 'error',
        message: error.message,
        status: error.status,
        traceId: error.traceId,
      }
    }

    throw error
  }
}
