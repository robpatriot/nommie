import { describe, expect, it, vi } from 'vitest'
import { QueryClient } from '@tanstack/react-query'

import { queryKeys } from '@/lib/queries/query-keys'
import {
  defaultLwCacheState,
  onLongWaitInvalidated,
  onOptimisticSend,
  onYourTurn,
  requestLwRefetch,
  type LwCacheState,
} from '@/lib/queries/lw-cache'

const mocks = vi.hoisted(() => ({
  getWaitingLongestGameAction: vi.fn(),
}))

vi.mock('@/app/actions/game-actions', async (importOriginal) => {
  const actual = (await importOriginal()) as Record<string, unknown>
  return {
    ...actual,
    getWaitingLongestGameAction: mocks.getWaitingLongestGameAction,
  }
})

function createClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: Infinity },
      mutations: { retry: false },
    },
  })
}

function seed(client: QueryClient, patch: Partial<LwCacheState>) {
  client.setQueryData<LwCacheState>(queryKeys.games.waitingLongestCache(), {
    ...defaultLwCacheState(),
    ...patch,
  })
}

function getState(client: QueryClient): LwCacheState {
  return (
    client.getQueryData<LwCacheState>(queryKeys.games.waitingLongestCache()) ??
    defaultLwCacheState()
  )
}

describe('lw-cache', () => {
  it('R0: coalesces overlapping refetches into inFlight + one dirty follow-up (last-response-wins)', async () => {
    const client = createClient()

    let resolveFirst: ((v: any) => void) | undefined

    mocks.getWaitingLongestGameAction
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveFirst = resolve
          })
      )
      // Follow-up refetch should happen automatically; make it resolve immediately.
      .mockResolvedValueOnce({ kind: 'ok', data: [9] })

    // Start first refetch (in flight).
    const p1 = requestLwRefetch(client, { createSnapshot: false })

    // Request a second refetch while first is in flight -> should only mark dirty.
    await requestLwRefetch(client, { createSnapshot: false })

    // Resolve first response, which should trigger exactly one follow-up.
    resolveFirst?.({ kind: 'ok', data: [1, 2, 3] })

    await p1

    expect(mocks.getWaitingLongestGameAction).toHaveBeenCalledTimes(2)

    expect(getState(client).pool).toEqual([9])
  })

  it('R1: long_wait_invalidated clears snapshot and refetches', async () => {
    const client = createClient()
    seed(client, {
      pool: [10, 11],
      isCompleteFromServer: true,
      snapshot: { gameId: 42, pool: [100], isCompleteFromServer: true },
    })

    mocks.getWaitingLongestGameAction.mockResolvedValueOnce({
      kind: 'ok',
      data: [3],
    })
    await onLongWaitInvalidated(client)

    expect(getState(client).snapshot).toBeUndefined()
    expect(getState(client).pool).toEqual([3])
  })

  it('R2: optimistic send removes current game from pool', () => {
    const client = createClient()
    seed(client, {
      pool: [10, 99, 11],
      isCompleteFromServer: true,
    })

    onOptimisticSend(client, { gameId: 99 })
    expect(getState(client).pool).toEqual([10, 11])
  })

  it('R2: optimistic send clears snapshot when acting in a different game than the snapshot', () => {
    const client = createClient()
    seed(client, {
      pool: [10, 99, 11],
      isCompleteFromServer: true,
      snapshot: { gameId: 42, pool: [100], isCompleteFromServer: true },
    })

    onOptimisticSend(client, { gameId: 99 })
    expect(getState(client).snapshot).toBeUndefined()
  })

  it('R2: optimistic send leaves snapshot untouched when acting in the snapshot game', () => {
    const client = createClient()
    seed(client, {
      pool: [99, 10],
      isCompleteFromServer: true,
      snapshot: { gameId: 99, pool: [1, 2], isCompleteFromServer: true },
    })

    onOptimisticSend(client, { gameId: 99 })
    expect(getState(client).snapshot).toEqual({
      gameId: 99,
      pool: [1, 2],
      isCompleteFromServer: true,
    })
  })

  it('R2: optimistic send conditionally refetches when pool is small and server may be truncated', async () => {
    const client = createClient()
    seed(client, {
      pool: [99],
      isCompleteFromServer: false,
    })

    mocks.getWaitingLongestGameAction.mockResolvedValueOnce({
      kind: 'ok',
      data: [7, 8],
    })
    onOptimisticSend(client, { gameId: 99 })

    await expect
      .poll(() => mocks.getWaitingLongestGameAction.mock.calls.length)
      .toBe(1)
    await expect.poll(() => getState(client).pool).toEqual([7, 8])
  })

  it('R4: your_turn restores snapshot when it matches and is populated', async () => {
    const client = createClient()
    seed(client, {
      pool: [10, 11],
      isCompleteFromServer: false,
      snapshot: { gameId: 42, pool: [501, 502], isCompleteFromServer: true },
    })

    await onYourTurn(client, { gameId: 42 })

    expect(getState(client).pool).toEqual([501, 502])
    expect(mocks.getWaitingLongestGameAction).toHaveBeenCalledTimes(0)
  })

  it('R4: your_turn adds locally (no refetch) when pool has < 2 games', async () => {
    const client = createClient()
    seed(client, {
      pool: [10],
      isCompleteFromServer: false,
    })

    await onYourTurn(client, { gameId: 42 })

    expect(mocks.getWaitingLongestGameAction).toHaveBeenCalledTimes(0)
    expect(getState(client).pool).toEqual([10, 42])
  })

  it('R4: your_turn (local add) clears snapshot if snapshot is for a different game', async () => {
    const client = createClient()
    seed(client, {
      pool: [10],
      isCompleteFromServer: false,
      snapshot: { gameId: 99, pool: [1, 2], isCompleteFromServer: true },
    })

    await onYourTurn(client, { gameId: 42 })

    expect(getState(client).pool).toEqual([10, 42])
    expect(getState(client).snapshot).toBeUndefined()
  })

  it('R4: your_turn restores snapshot when it matches (no refetch)', async () => {
    const client = createClient()
    seed(client, {
      pool: [10, 11],
      isCompleteFromServer: false,
      snapshot: { gameId: 42, pool: [501, 502], isCompleteFromServer: true },
    })

    await onYourTurn(client, { gameId: 42 })

    expect(mocks.getWaitingLongestGameAction).toHaveBeenCalledTimes(0)
    expect(getState(client).pool).toEqual([501, 502])
    expect(getState(client).snapshot?.gameId).toBe(42)
  })

  it('R4: your_turn refetches and creates snapshot when pool is >= 2 and snapshot does not match', async () => {
    const client = createClient()
    seed(client, {
      pool: [10, 11],
      isCompleteFromServer: false,
      snapshot: { gameId: 99, pool: [501, 502], isCompleteFromServer: true },
    })

    mocks.getWaitingLongestGameAction.mockResolvedValueOnce({
      kind: 'ok',
      data: [9],
    })
    await onYourTurn(client, { gameId: 42 })

    expect(mocks.getWaitingLongestGameAction).toHaveBeenCalledTimes(1)
    expect(getState(client).pool).toEqual([9])
    expect(getState(client).snapshot).toEqual({
      gameId: 42,
      pool: [9],
      isCompleteFromServer: true,
    })
  })
})
