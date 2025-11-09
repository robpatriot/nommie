'use server'

import { BackendApiError } from '@/lib/api'
import {
  fetchGameSnapshot,
  fetchSeatDisplayNames,
  markPlayerReady,
} from '@/lib/api/game-room'
import { DEFAULT_VIEWER_SEAT } from '@/lib/game-room/constants'
import type { Card, GameSnapshot, Seat } from '@/lib/game-room/types'

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

    const playerNames = await fetchSeatDisplayNames(request.gameId)

    return {
      kind: 'ok',
      data: {
        snapshot: snapshotResult.snapshot,
        etag: snapshotResult.etag,
        playerNames,
        viewerSeat: DEFAULT_VIEWER_SEAT,
        viewerHand: [],
        timestamp: new Date().toISOString(),
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
