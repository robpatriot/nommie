'use server'

import { getTranslations } from 'next-intl/server'
import { BackendApiError } from '@/lib/api'
import {
  fetchGameSnapshot,
  markPlayerReady,
  leaveGame,
  rejoinGame,
  submitBid,
  selectTrump,
  submitPlay,
  addAiSeat,
  updateAiSeat,
  removeAiSeat,
  listRegisteredAis,
  type AiRegistryResponse,
} from '@/lib/api/game-room'
import {
  toErrorResult,
  type ActionResult,
  type SimpleActionResult,
  type SnapshotActionResult,
} from '@/lib/api/action-helpers'
import { validateSeat } from '@/utils/seat-validation'
import type {
  BidConstraints,
  Card,
  GameSnapshot,
  Seat,
  Trump,
} from '@/lib/game-room/types'
import { gameStateMsgToSnapshotPayload } from '@/lib/game-room/protocol/transform'

export interface GameRoomSnapshotRequest {
  gameId: number
  etag?: string
}

export type GameRoomSnapshotActionResult =
  SnapshotActionResult<GameRoomSnapshotPayload>

export interface GameRoomSnapshotPayload {
  snapshot: GameSnapshot
  etag?: string
  version?: number
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

    const payload = gameStateMsgToSnapshotPayload(snapshotResult.msg, {
      etag: snapshotResult.etag,
      timestamp: new Date().toISOString(),
    })

    return {
      kind: 'ok',
      data: payload,
    }
  } catch (error) {
    // Handle not_modified case (304 status)
    if (error instanceof BackendApiError && error.status === 304) {
      return { kind: 'not_modified' }
    }
    const t = await getTranslations('errors.actions')
    return toErrorResult(error, t('failedToFetchGameSnapshot'))
  }
}

export async function markPlayerReadyAction(
  gameId: number,
  isReady: boolean,
  version: number
): Promise<SimpleActionResult> {
  try {
    await markPlayerReady(gameId, isReady, version)
    return { kind: 'ok' }
  } catch (error) {
    const t = await getTranslations('errors.actions')
    return toErrorResult(error, t('failedToMarkPlayerReady'))
  }
}

export async function leaveGameAction(
  gameId: number,
  version?: number
): Promise<SimpleActionResult> {
  try {
    // If no version is provided, fetch the game snapshot to get it
    let finalVersion = version
    if (finalVersion === undefined) {
      const snapshotResult = await getGameRoomSnapshotAction({ gameId })
      if (
        snapshotResult.kind === 'ok' &&
        snapshotResult.data.version !== undefined
      ) {
        finalVersion = snapshotResult.data.version
      }
    }

    if (finalVersion === undefined) {
      const t = await getTranslations('errors.actions')
      return toErrorResult(
        new Error('Lock version required'),
        t('failedToLeaveGameNoVersion')
      )
    }

    await leaveGame(gameId, finalVersion)
    return { kind: 'ok' }
  } catch (error) {
    const t = await getTranslations('errors.actions')
    return toErrorResult(error, t('failedToLeaveGame'))
  }
}

export interface RejoinGameRequest {
  gameId: number
  version?: number
}

export async function rejoinGameAction(
  request: RejoinGameRequest
): Promise<SimpleActionResult> {
  try {
    let finalVersion = request.version
    if (finalVersion === undefined) {
      const snapshotResult = await getGameRoomSnapshotAction({
        gameId: request.gameId,
      })
      if (
        snapshotResult.kind === 'ok' &&
        snapshotResult.data.version !== undefined
      ) {
        finalVersion = snapshotResult.data.version
      }
    }

    if (finalVersion === undefined) {
      const t = await getTranslations('errors.actions')
      return toErrorResult(
        new Error('Lock version required'),
        t('failedToRejoinGameNoVersion')
      )
    }

    await rejoinGame(request.gameId, finalVersion)
    return { kind: 'ok' }
  } catch (error) {
    const t = await getTranslations('errors.actions')
    return toErrorResult(error, t('failedToRejoinGame'))
  }
}

export interface SubmitBidRequest {
  gameId: number
  bid: number
  version: number
}

export async function submitBidAction(
  request: SubmitBidRequest
): Promise<SimpleActionResult> {
  const bidValue = Math.trunc(request.bid)

  if (!Number.isFinite(bidValue) || bidValue < 0) {
    const t = await getTranslations('errors.validation')
    return {
      kind: 'error',
      message: t('invalidBidValue'),
      status: 400,
    }
  }

  if (!Number.isFinite(request.version) || request.version < 0) {
    const t = await getTranslations('errors.validation')
    return {
      kind: 'error',
      message: t('invalidVersion'),
      status: 400,
    }
  }

  try {
    await submitBid(request.gameId, bidValue, request.version)
    return { kind: 'ok' }
  } catch (error) {
    const t = await getTranslations('errors.actions')
    return toErrorResult(error, t('failedToSubmitBid'))
  }
}

export interface SelectTrumpRequest {
  gameId: number
  trump: Trump
  version: number
}

export async function selectTrumpAction(
  request: SelectTrumpRequest
): Promise<SimpleActionResult> {
  if (!Number.isFinite(request.version) || request.version < 0) {
    const t = await getTranslations('errors.validation')
    return {
      kind: 'error',
      message: t('invalidVersion'),
      status: 400,
    }
  }

  try {
    await selectTrump(request.gameId, request.trump, request.version)
    return { kind: 'ok' }
  } catch (error) {
    const t = await getTranslations('errors.actions')
    return toErrorResult(error, t('failedToSelectTrump'))
  }
}

export interface SubmitPlayRequest {
  gameId: number
  card: string
  version: number
}

export async function submitPlayAction(
  request: SubmitPlayRequest
): Promise<SimpleActionResult> {
  const card = request.card.trim()
  const t = await getTranslations('errors.validation')

  if (!card) {
    return {
      kind: 'error',
      message: t('cardSelectionRequired'),
      status: 400,
      code: 'VALIDATION_ERROR',
    }
  }

  if (!Number.isFinite(request.version) || request.version < 0) {
    return {
      kind: 'error',
      message: t('invalidVersion'),
      status: 400,
    }
  }

  try {
    await submitPlay(request.gameId, card, request.version)
    return { kind: 'ok' }
  } catch (error) {
    const t = await getTranslations('errors.actions')
    return toErrorResult(error, t('failedToPlayCard'))
  }
}

export interface ManageAiSeatRequest {
  gameId: number
  seat?: Seat
  registryName?: string
  registryVersion?: string
  seed?: number
  version?: number
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

  // If no version is provided, fetch the game snapshot to get it
  let finalVersion = request.version
  if (finalVersion === undefined) {
    try {
      const snapshotResult = await fetchGameSnapshot(request.gameId)
      if (snapshotResult.kind === 'ok') {
        finalVersion = snapshotResult.msg.version
      } else {
        const t = await getTranslations('errors.actions')
        return toErrorResult(
          new Error('Failed to get lock version from game snapshot'),
          t('failedToAddAiSeatNoVersion')
        )
      }
    } catch (error) {
      const t = await getTranslations('errors.actions')
      return toErrorResult(error, t('failedToAddAiSeatNoSnapshot'))
    }
  }

  try {
    await addAiSeat(request.gameId, finalVersion, {
      seat: request.seat,
      registryName: request.registryName,
      registryVersion: request.registryVersion,
      seed: request.seed,
    })
    return { kind: 'ok' }
  } catch (error) {
    const t = await getTranslations('errors.actions')
    return toErrorResult(error, t('failedToAddAiSeat'))
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

  // If no version is provided, fetch the game snapshot to get it
  let finalVersion = request.version
  if (finalVersion === undefined) {
    try {
      const snapshotResult = await fetchGameSnapshot(request.gameId)
      if (snapshotResult.kind === 'ok') {
        finalVersion = snapshotResult.msg.version
      } else {
        const t = await getTranslations('errors.actions')
        return toErrorResult(
          new Error('Failed to get lock version from game snapshot'),
          t('failedToRemoveAiSeatNoVersion')
        )
      }
    } catch (error) {
      const t = await getTranslations('errors.actions')
      return toErrorResult(error, t('failedToRemoveAiSeatNoSnapshot'))
    }
  }

  try {
    await removeAiSeat(request.gameId, finalVersion, {
      seat: request.seat,
    })
    return { kind: 'ok' }
  } catch (error) {
    const t = await getTranslations('errors.actions')
    return toErrorResult(error, t('failedToRemoveAiSeat'))
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

  // If no version is provided, fetch the game snapshot to get it
  let finalVersion = request.version
  if (finalVersion === undefined) {
    try {
      const snapshotResult = await fetchGameSnapshot(request.gameId)
      if (snapshotResult.kind === 'ok') {
        finalVersion = snapshotResult.msg.version
      } else {
        const t = await getTranslations('errors.actions')
        return toErrorResult(
          new Error('Failed to get lock version from game snapshot'),
          t('failedToUpdateAiSeatNoVersion')
        )
      }
    } catch (error) {
      const t = await getTranslations('errors.actions')
      return toErrorResult(error, t('failedToUpdateAiSeatNoSnapshot'))
    }
  }

  try {
    await updateAiSeat(request.gameId, finalVersion, {
      seat: request.seat,
      registryName: request.registryName,
      registryVersion: request.registryVersion,
      seed: request.seed,
    })
    return { kind: 'ok' }
  } catch (error) {
    const t = await getTranslations('errors.actions')
    return toErrorResult(error, t('failedToUpdateAiSeat'))
  }
}

export async function fetchAiRegistryAction(): Promise<
  ActionResult<AiRegistryResponse>
> {
  try {
    const registry = await listRegisteredAis()
    return { kind: 'ok', data: registry }
  } catch (error) {
    const t = await getTranslations('errors.actions')
    return toErrorResult(error, t('failedToFetchAiRegistry'))
  }
}
