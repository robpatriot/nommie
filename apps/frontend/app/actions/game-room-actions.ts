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
import {
  toErrorResult,
  type ActionResult,
  type SimpleActionResult,
  type SnapshotActionResult,
} from '@/lib/api/action-helpers'
import { DEFAULT_VIEWER_SEAT } from '@/lib/game-room/constants'
import { extractPlayerNames } from '@/utils/player-names'
import { validateSeat } from '@/utils/seat-validation'
import type {
  BidConstraints,
  Card,
  GameSnapshot,
  Seat,
  Trump,
} from '@/lib/game-room/types'

export interface GameRoomSnapshotRequest {
  gameId: number
  etag?: string
}

export type GameRoomSnapshotActionResult =
  SnapshotActionResult<GameRoomSnapshotPayload>

export interface GameRoomSnapshotPayload {
  snapshot: GameSnapshot
  etag?: string
  lockVersion?: number
  playerNames: [string, string, string, string]
  viewerSeat: Seat | null
  viewerHand: Card[]
  timestamp: string
  hostSeat: Seat
  bidConstraints?: BidConstraints | null
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
    const playerNames = extractPlayerNames(seating)

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
        lockVersion: snapshotResult.lockVersion,
        playerNames,
        viewerSeat,
        viewerHand: snapshotResult.viewerHand ?? [],
        timestamp: new Date().toISOString(),
        hostSeat,
        bidConstraints: snapshotResult.bidConstraints ?? null,
      },
    }
  } catch (error) {
    // Handle not_modified case (304 status)
    if (error instanceof BackendApiError && error.status === 304) {
      return { kind: 'not_modified' }
    }
    return toErrorResult(error, 'Failed to fetch game snapshot')
  }
}

export async function markPlayerReadyAction(
  gameId: number
): Promise<SimpleActionResult> {
  try {
    await markPlayerReady(gameId)
    return { kind: 'ok' }
  } catch (error) {
    return toErrorResult(error, 'Failed to mark player ready')
  }
}

export interface SubmitBidRequest {
  gameId: number
  bid: number
  lockVersion: number
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

  if (!Number.isFinite(request.lockVersion) || request.lockVersion < 0) {
    return {
      kind: 'error',
      message: 'Invalid lock version',
      status: 400,
    }
  }

  try {
    await submitBid(request.gameId, bidValue, request.lockVersion)
    return { kind: 'ok' }
  } catch (error) {
    return toErrorResult(error, 'Failed to submit bid')
  }
}

export interface SelectTrumpRequest {
  gameId: number
  trump: Trump
  lockVersion: number
}

export async function selectTrumpAction(
  request: SelectTrumpRequest
): Promise<SimpleActionResult> {
  if (!Number.isFinite(request.lockVersion) || request.lockVersion < 0) {
    return {
      kind: 'error',
      message: 'Invalid lock version',
      status: 400,
    }
  }

  try {
    await selectTrump(request.gameId, request.trump, request.lockVersion)
    return { kind: 'ok' }
  } catch (error) {
    return toErrorResult(error, 'Failed to select trump')
  }
}

export interface SubmitPlayRequest {
  gameId: number
  card: string
  lockVersion: number
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
      code: 'VALIDATION_ERROR',
    }
  }

  if (!Number.isFinite(request.lockVersion) || request.lockVersion < 0) {
    return {
      kind: 'error',
      message: 'Invalid lock version',
      status: 400,
    }
  }

  try {
    await submitPlay(request.gameId, card, request.lockVersion)
    return { kind: 'ok' }
  } catch (error) {
    return toErrorResult(error, 'Failed to play card')
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
  const seatError = validateSeat(request.seat, false)
  if (seatError) {
    return {
      kind: 'error',
      message: seatError,
      status: 400,
      code: 'VALIDATION_ERROR',
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
    return toErrorResult(error, 'Failed to add AI seat')
  }
}

export async function removeAiSeatAction(
  request: ManageAiSeatRequest
): Promise<SimpleActionResult> {
  const seatError = validateSeat(request.seat, false)
  if (seatError) {
    return {
      kind: 'error',
      message: seatError,
      status: 400,
      code: 'VALIDATION_ERROR',
    }
  }

  try {
    await removeAiSeat(request.gameId, {
      seat: request.seat,
    })
    return { kind: 'ok' }
  } catch (error) {
    return toErrorResult(error, 'Failed to remove AI seat')
  }
}

export async function updateAiSeatAction(
  request: ManageAiSeatRequest
): Promise<SimpleActionResult> {
  const seatError = validateSeat(request.seat, true)
  if (seatError) {
    return {
      kind: 'error',
      message: seatError,
      status: 400,
      code: 'VALIDATION_ERROR',
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
    return toErrorResult(error, 'Failed to update AI seat')
  }
}

export async function fetchAiRegistryAction(): Promise<
  ActionResult<AiRegistryEntry[]>
> {
  try {
    const ais = await listRegisteredAis()
    return { kind: 'ok', data: ais }
  } catch (error) {
    return toErrorResult(error, 'Failed to fetch AI registry')
  }
}
