import type { QueryClient } from '@tanstack/react-query'
import { act } from '@testing-library/react'
import { waitFor } from '@testing-library/react'
import { expect } from 'vitest'
import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import { initSnapshotFixture } from '../mocks/game-snapshot'
import {
  MockWebSocket,
  mockWebSocketInstances,
} from '@/test/setup/mock-websocket'
import type { Seat } from '@/lib/game-room/types'
import { isValidSeat } from '@/utils/seat-validation'

/**
 * Creates initial game room snapshot data for tests.
 * Most GameRoomClient tests use this variant (simple etag).
 */
export function createInitialData(
  snapshot = initSnapshotFixture,
  overrides?: Partial<GameRoomSnapshotPayload>
): GameRoomSnapshotPayload {
  return {
    snapshot,
    etag: 'initial-etag',
    playerNames: ['Alex', 'Bailey', 'Casey', 'Dakota'],
    viewerSeat: 0,
    viewerHand: [],
    timestamp: new Date().toISOString(),
    hostSeat: 0,
    ...overrides,
  }
}

/**
 * Creates initial game room snapshot data with versioned etag.
 * Used by useGameSync tests which need version tracking.
 */
export function createInitialDataWithVersion(
  gameId: number,
  version = 1,
  overrides?: Partial<GameRoomSnapshotPayload>
): GameRoomSnapshotPayload {
  return {
    snapshot: initSnapshotFixture,
    etag: `"game-${gameId}-v${version}"`,
    version,
    playerNames: ['Alex', 'Bailey', 'Casey', 'Dakota'],
    viewerSeat: 0,
    viewerHand: [],
    timestamp: new Date().toISOString(),
    hostSeat: 0,
    bidConstraints: null,
    ...overrides,
  }
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
 * Sends a WebSocket snapshot message and updates the query cache.
 * This simulates what useGameSync does when receiving snapshot messages.
 */
export function sendWebSocketSnapshot(
  ws: MockWebSocket,
  snapshot: typeof initSnapshotFixture,
  gameId: number,
  queryClient: QueryClient,
  overrides?: {
    viewerSeat?: number
    version?: number
    viewerHand?: string[]
  }
): void {
  // Transform the game_state message to GameRoomSnapshotPayload format
  // This simulates what useGameSync.transformGameStateMessage does
  const version = overrides?.version ?? 1
  const viewerSeatRaw = overrides?.viewerSeat ?? 0
  const viewerSeat: Seat | null =
    viewerSeatRaw !== null &&
    viewerSeatRaw !== undefined &&
    isValidSeat(viewerSeatRaw)
      ? (viewerSeatRaw as Seat)
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
