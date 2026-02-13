import type { QueryClient } from '@tanstack/react-query'
import { act } from '@testing-library/react'
import { waitFor } from '@testing-library/react'
import { expect } from 'vitest'
import type { GameRoomState } from '@/lib/game-room/state'
import type { Seat } from '@/lib/game-room/types'
import { gameStateMsgToRoomState } from '@/lib/game-room/state'
import { initSnapshotFixture } from '../mocks/game-snapshot'
import {
  MockWebSocket,
  mockWebSocketInstances,
} from '@/test/setup/mock-websocket'
import { isValidSeat } from '@/utils/seat-validation'

type PayloadOverrides = {
  etag?: string
  version?: number
  viewerSeat?: number | null
  viewerHand?: string[]
}

function buildStateFromMsg(
  gameId: number,
  snapshot: typeof initSnapshotFixture,
  opts: {
    version?: number
    viewerSeat?: number | null
    viewerHand?: string[]
    etag?: string
  }
): GameRoomState {
  const msg = {
    type: 'game_state' as const,
    topic: { kind: 'game' as const, id: gameId },
    version: opts.version ?? 1,
    game: snapshot,
    viewer: {
      seat: (opts.viewerSeat ?? 0) as Seat | null,
      hand: opts.viewerHand ?? [],
      bidConstraints: null,
    },
  }
  const state = gameStateMsgToRoomState(msg, { source: 'http' })
  return opts.etag ? { ...state, etag: opts.etag } : state
}

/**
 * Creates state for mocking getGameRoomStateAction (returns GameRoomState).
 */
export function createStateForMock(
  gameId: number,
  snapshot = initSnapshotFixture,
  overrides?: PayloadOverrides
): GameRoomState {
  return buildStateFromMsg(gameId, snapshot, {
    version: overrides?.version ?? 1,
    viewerSeat: overrides?.viewerSeat ?? 0,
    viewerHand: overrides?.viewerHand,
    etag: overrides?.etag ?? 'initial-etag',
  })
}

/**
 * Creates initial game room state for tests.
 */
export function createInitialState(
  gameId: number,
  snapshot = initSnapshotFixture,
  overrides?: PayloadOverrides & Partial<Pick<GameRoomState, 'etag'>>
): GameRoomState {
  return buildStateFromMsg(gameId, snapshot, {
    version: overrides?.version ?? 1,
    viewerSeat: overrides?.viewerSeat ?? 0,
    viewerHand: overrides?.viewerHand,
    etag: overrides?.etag ?? 'initial-etag',
  })
}

/**
 * Creates initial state with versioned etag.
 * Used by useGameSync tests which need version tracking.
 */
export function createInitialStateWithVersion(
  gameId: number,
  version = 1,
  overrides?: PayloadOverrides
): GameRoomState {
  return buildStateFromMsg(gameId, initSnapshotFixture, {
    version,
    viewerSeat: overrides?.viewerSeat ?? 0,
    viewerHand: overrides?.viewerHand,
    etag: overrides?.etag ?? `"game-${gameId}-v${version}"`,
  })
}

/**
 * Creates state with version for mocking getGameRoomStateAction.
 */
export function createStateWithVersionForMock(
  gameId: number,
  version = 1,
  overrides?: PayloadOverrides
): GameRoomState {
  return createInitialStateWithVersion(gameId, version, overrides)
}

function getSentJson(ws: MockWebSocket): unknown[] {
  return ws.sent.map((s) => {
    try {
      return JSON.parse(s) as unknown
    } catch {
      return s
    }
  })
}

function hasSentType(ws: MockWebSocket, type: string): boolean {
  return getSentJson(ws).some(
    (m) => typeof m === 'object' && m !== null && (m as any).type === type
  )
}

function serverSendJson(ws: MockWebSocket, msg: unknown) {
  ws.onmessage?.(
    new MessageEvent('message', {
      data: JSON.stringify(msg),
    })
  )
}

/**
 * Waits for a WebSocket connection to be established AND handshake completed.
 * Returns the first connected WebSocket instance.
 */
export async function waitForWebSocketConnection(): Promise<MockWebSocket> {
  await waitFor(
    () => {
      expect(mockWebSocketInstances.length).toBeGreaterThan(0)
      const ws = mockWebSocketInstances[0]
      expect(ws.readyState).toBe(MockWebSocket.OPEN)
    },
    { timeout: 2000 }
  )

  const ws = mockWebSocketInstances[0]

  // Wait for client hello (sent in ws.onopen)
  await waitFor(
    () => {
      expect(hasSentType(ws, 'hello')).toBe(true)
    },
    { timeout: 2000 }
  )

  // Complete handshake - wrap in act to prevent warnings from WebSocketProvider state updates
  await act(async () => {
    serverSendJson(ws, { type: 'hello_ack', protocol: 1, user_id: 123 })
    // Allow state updates to flush
    await new Promise((resolve) => setTimeout(resolve, 0))
  })

  return ws
}

/**
 * Sends a WebSocket game_state message.
 * Simulates what useGameSync receives and processes.
 */
export function sendWebSocketSnapshot(
  ws: MockWebSocket,
  snapshot: typeof initSnapshotFixture,
  gameId: number,
  _queryClient: QueryClient,
  overrides?: {
    viewerSeat?: number
    version?: number
    viewerHand?: string[]
  }
): void {
  const version = overrides?.version ?? 1
  const viewerSeatRaw = overrides?.viewerSeat ?? 0
  const viewerSeat: number | null =
    viewerSeatRaw !== null &&
    viewerSeatRaw !== undefined &&
    isValidSeat(viewerSeatRaw)
      ? viewerSeatRaw
      : null
  const viewerHand = overrides?.viewerHand ?? []

  const message = {
    type: 'game_state',
    topic: { kind: 'game', id: gameId },
    version,
    game: snapshot,
    viewer: {
      seat: viewerSeat,
      hand: viewerHand,
      bidConstraints: null,
    },
  }

  act(() => {
    ws.onmessage?.(
      new MessageEvent('message', {
        data: JSON.stringify(message),
      })
    )
  })
}
