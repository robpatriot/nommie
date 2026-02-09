import { act, waitFor } from '@testing-library/react'
import type { QueryClient } from '@tanstack/react-query'
import { expect } from 'vitest'

import type { WireMsg } from '@/lib/game-room/protocol/types'
import { queryKeys } from '@/lib/queries/query-keys'
import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import { defaultLwCacheState, type LwCacheState } from '@/lib/queries/lw-cache'

import { mockWebSocketInstances } from '@/test/setup/mock-websocket'
import type { MockWebSocket } from '@/test/setup/mock-websocket'

export type SeedState = {
  snapshot?: {
    gameId: number
    payload: GameRoomSnapshotPayload
  }
  lwCache?: Pick<LwCacheState, 'pool' | 'isCompleteFromServer' | 'snapshot'>
}

export type ExpectedEffects = {
  snapshotVersion?: number
  lwRefetchCalls?: number
  lwPoolAfterRefetch?: number[]
  lwSnapshotGameIdAfter?: number | null
}

export type RealtimeScenario = {
  name: string
  gameId: number
  initialData: GameRoomSnapshotPayload
  seed?: SeedState
  msg: WireMsg
  expect: ExpectedEffects
}

function serverSendJson(ws: MockWebSocket, msg: unknown) {
  ws.onmessage?.(
    new MessageEvent('message', {
      data: JSON.stringify(msg),
    })
  )
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

function findSentByType<T extends { type: string }>(
  ws: MockWebSocket,
  type: string
): T | undefined {
  return getSentJson(ws).find(
    (m): m is T =>
      typeof m === 'object' && m !== null && (m as any).type === type
  )
}

async function waitForWsCount(n: number) {
  await waitFor(() => {
    expect(mockWebSocketInstances.length).toBe(n)
  })
}

async function waitForSentType(ws: MockWebSocket, type: string) {
  await waitFor(() => {
    expect(findSentByType(ws, type)).toBeDefined()
  })
}

/**
 * Establish provider hello handshake (hello -> hello_ack) and hook subscribe (subscribe -> ack).
 */
export async function connectAndSubscribe(ws: MockWebSocket, gameId: number) {
  await waitForSentType(ws, 'hello')

  await act(async () => {
    serverSendJson(ws, { type: 'hello_ack', protocol: 1, user_id: 123 })
  })

  await waitForSentType(ws, 'subscribe')
  const subscribe = findSentByType<{
    type: 'subscribe'
    topic: { kind: 'game'; id: number }
  }>(ws, 'subscribe')
  expect(subscribe?.topic).toEqual({ kind: 'game', id: gameId })

  await act(async () => {
    serverSendJson(ws, { type: 'ack', message: 'subscribed' })
  })
}

/**
 * Runs a realtime scenario against `useGameSync` + React Query cache.
 *
 * Notes:
 * - We intentionally seed queries AFTER handshake to avoid the WebSocketProvider's
 *   hello_ack reconnection refetch path from touching partially-configured queries.
 */
export async function runRealtimeScenario(
  scenario: RealtimeScenario,
  opts: {
    queryClient: QueryClient
    ws: MockWebSocket
    clearLwRefetchMock?: () => void
    getLwRefetchMockCallCount?: () => number
    setLwRefetchMockResponse?: (pool: number[]) => void
  }
) {
  const {
    queryClient,
    ws,
    clearLwRefetchMock,
    getLwRefetchMockCallCount,
    setLwRefetchMockResponse,
  } = opts

  // Ensure ws exists (provider auto-connects when authenticated).
  await waitForWsCount(1)

  await connectAndSubscribe(ws, scenario.gameId)

  // The provider performs an LW refetch on hello_ack. Clear that so scenario assertions
  // only reflect effects from the scenario message.
  clearLwRefetchMock?.()

  // Seed cache state (optional) after handshake
  if (scenario.seed?.snapshot) {
    const { gameId, payload } = scenario.seed.snapshot
    queryClient.setQueryData(queryKeys.games.snapshot(gameId), payload)
  }
  if (scenario.seed?.lwCache) {
    queryClient.setQueryData<LwCacheState>(
      queryKeys.games.waitingLongestCache(),
      {
        ...defaultLwCacheState(),
        ...scenario.seed.lwCache,
      }
    )
  }

  if (scenario.expect.lwPoolAfterRefetch) {
    setLwRefetchMockResponse?.(scenario.expect.lwPoolAfterRefetch)
  }

  // Send the message
  act(() => {
    serverSendJson(ws, scenario.msg)
  })

  // Assert LW refetch behavior (via mocked server action call count)
  if (
    typeof scenario.expect.lwRefetchCalls === 'number' &&
    typeof getLwRefetchMockCallCount === 'function'
  ) {
    await waitFor(() => {
      expect(getLwRefetchMockCallCount()).toBe(scenario.expect.lwRefetchCalls)
    })
  }

  if (scenario.expect.lwPoolAfterRefetch) {
    await waitFor(() => {
      const cached = queryClient.getQueryData<{
        pool: number[]
      }>(queryKeys.games.waitingLongestCache())
      expect(cached?.pool).toEqual(scenario.expect.lwPoolAfterRefetch)
    })
  }

  if (scenario.expect.lwSnapshotGameIdAfter !== undefined) {
    await waitFor(() => {
      const cached = queryClient.getQueryData<{
        snapshot?: { gameId: number } | undefined
      }>(queryKeys.games.waitingLongestCache())
      const actual = cached?.snapshot?.gameId ?? null
      expect(actual).toBe(scenario.expect.lwSnapshotGameIdAfter)
    })
  }

  // Assert snapshot version if requested
  if (scenario.expect.snapshotVersion !== undefined) {
    await waitFor(() => {
      const cached = queryClient.getQueryData<GameRoomSnapshotPayload>(
        queryKeys.games.snapshot(scenario.gameId)
      )
      expect(cached?.version).toBe(scenario.expect.snapshotVersion)
    })
  }
}
