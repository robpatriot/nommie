import { getWaitingLongestGameAction } from '@/app/actions/game-actions'
import { queryKeys } from '@/lib/queries/query-keys'

import type { QueryClient } from '@tanstack/react-query'

export type LwSnapshot = {
  gameId: number
  pool: number[]
  isCompleteFromServer: boolean
}

export function getLwPendingAction(
  queryClient: QueryClient,
  gameId: number
): boolean {
  return (
    queryClient.getQueryData<boolean>(
      queryKeys.games.waitingLongestPendingAction(gameId)
    ) ?? false
  )
}

export function setLwPendingAction(
  queryClient: QueryClient,
  gameId: number,
  pendingAction: boolean
): void {
  queryClient.setQueryData<boolean>(
    queryKeys.games.waitingLongestPendingAction(gameId),
    pendingAction
  )
}

export type LwCacheState = {
  pool: number[]
  /**
   * Set only from a server refetch result. If false, the server might have more
   * games than we hold locally (truncated response).
   */
  isCompleteFromServer: boolean
  snapshot?: LwSnapshot

  // mechanical refetch state (R0)
  refetchInFlight: boolean
  refetchDirty: boolean
  refetchRequestId: number
}

export const LW_MAX_POOL_SIZE = 5

export function defaultLwCacheState(): LwCacheState {
  return {
    pool: [],
    // conservative default: until a server response says “complete”, assume it may be truncated.
    isCompleteFromServer: false,
    snapshot: undefined,
    refetchInFlight: false,
    refetchDirty: false,
    refetchRequestId: 0,
  }
}

export function getLwCacheState(queryClient: QueryClient): LwCacheState {
  return (
    queryClient.getQueryData<LwCacheState>(
      queryKeys.games.waitingLongestCache()
    ) ?? defaultLwCacheState()
  )
}

function setLwCacheState(
  queryClient: QueryClient,
  updater: (prev: LwCacheState) => LwCacheState
) {
  queryClient.setQueryData<LwCacheState>(
    queryKeys.games.waitingLongestCache(),
    (prev) => updater(prev ?? defaultLwCacheState())
  )
}

function normalizeServerPool(pool: number[]): number[] {
  const unique = Array.from(new Set(pool.filter((id) => Number.isFinite(id))))
  return unique.slice(0, LW_MAX_POOL_SIZE)
}

function restoreFromSnapshot(
  queryClient: QueryClient,
  snapshot: LwSnapshot
): void {
  setLwCacheState(queryClient, (prev) => ({
    ...prev,
    pool: snapshot.pool,
    isCompleteFromServer: snapshot.isCompleteFromServer,
  }))
}

async function doRefetch(
  queryClient: QueryClient,
  opts: { requestId: number; createSnapshot: boolean; snapshotGameId?: number }
): Promise<void> {
  const res = await getWaitingLongestGameAction()
  if (res.kind !== 'ok') {
    // Conservative: mark in-flight false so future events can retry.
    setLwCacheState(queryClient, (prev) => ({
      ...prev,
      refetchInFlight: false,
    }))
    return
  }

  const pool = normalizeServerPool(res.data)
  const isCompleteFromServer = pool.length < LW_MAX_POOL_SIZE

  setLwCacheState(queryClient, (prev) => {
    // last-response-wins (R0.2)
    if (opts.requestId !== prev.refetchRequestId) {
      return prev
    }

    const next: LwCacheState = {
      ...prev,
      pool,
      isCompleteFromServer,
      refetchInFlight: false,
    }

    if (opts.createSnapshot && typeof opts.snapshotGameId === 'number') {
      next.snapshot = {
        gameId: opts.snapshotGameId,
        pool,
        isCompleteFromServer,
      }
    }

    return next
  })

  // If another refetch was requested while this one was in flight, run exactly one follow-up.
  const after = getLwCacheState(queryClient)
  if (after.refetchDirty) {
    setLwCacheState(queryClient, (prev) => ({
      ...prev,
      refetchDirty: false,
    }))
    // follow-up refetch never creates a snapshot (R0.2)
    await requestLwRefetch(queryClient, { createSnapshot: false })
  }
}

export async function requestLwRefetch(
  queryClient: QueryClient,
  opts: { createSnapshot: boolean; snapshotGameId?: number }
): Promise<void> {
  const state = getLwCacheState(queryClient)
  if (state.refetchInFlight) {
    setLwCacheState(queryClient, (prev) => ({
      ...prev,
      refetchDirty: true,
    }))
    return
  }

  const requestId = state.refetchRequestId + 1
  setLwCacheState(queryClient, (prev) => ({
    ...prev,
    refetchInFlight: true,
    refetchRequestId: requestId,
  }))

  await doRefetch(queryClient, {
    requestId,
    createSnapshot: opts.createSnapshot,
    snapshotGameId: opts.snapshotGameId,
  })
}

export async function onLongWaitInvalidated(
  queryClient: QueryClient
): Promise<void> {
  setLwCacheState(queryClient, (prev) => ({
    ...prev,
    snapshot: undefined,
  }))
  await requestLwRefetch(queryClient, { createSnapshot: false })
}

/**
 * `your_turn` handler.
 */
export async function onYourTurn(
  queryClient: QueryClient,
  opts: { gameId: number }
): Promise<void> {
  const state = getLwCacheState(queryClient)

  // R4: if already present, no-op
  if (state.pool.includes(opts.gameId)) return

  // R4: navigation mode (≤ 2 games => ordering irrelevant)
  if (state.pool.length < 2) {
    setLwCacheState(queryClient, (prev) => ({
      ...prev,
      pool: normalizeServerPool([...prev.pool, opts.gameId]),
      snapshot:
        prev.snapshot && prev.snapshot.gameId !== opts.gameId
          ? undefined
          : prev.snapshot,
    }))
    return
  }

  // R4: snapshot reuse only when correctness is provable
  if (state.snapshot && state.snapshot.gameId === opts.gameId) {
    restoreFromSnapshot(queryClient, state.snapshot)
    return
  }

  // Otherwise, refetch and create a snapshot tied to this game.
  await requestLwRefetch(queryClient, {
    createSnapshot: true,
    snapshotGameId: opts.gameId,
  })
}

/**
 * Optimistic send (user takes a turn).
 */
export function onOptimisticSend(
  queryClient: QueryClient,
  opts: { gameId: number }
): void {
  setLwPendingAction(queryClient, opts.gameId, true)

  // R2: removing a game is always safe and preserves relative ordering.
  setLwCacheState(queryClient, (prev) => {
    const next: LwCacheState = {
      ...prev,
      pool: prev.pool.filter((id) => id !== opts.gameId),
      snapshot:
        prev.snapshot && prev.snapshot.gameId !== opts.gameId
          ? undefined
          : prev.snapshot,
    }

    return next
  })

  const after = getLwCacheState(queryClient)
  if (after.pool.length < 2 && after.isCompleteFromServer === false) {
    // Fire-and-forget: this is navigation-only state.
    void requestLwRefetch(queryClient, { createSnapshot: false })
  }
}

export function renderNavigationPool(opts: {
  pool: number[]
  currentGameId?: number
}): number[] {
  return typeof opts.currentGameId === 'number'
    ? opts.pool.filter((id) => id !== opts.currentGameId)
    : opts.pool
}
