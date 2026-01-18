/**
 * Centralized query key factory for TanStack Query.
 * All query keys should be generated through these factories.
 *
 * Query keys are hierarchical arrays that allow for:
 * - Invalidating related queries (e.g., invalidate all games queries)
 * - Precise cache invalidation (e.g., invalidate specific game snapshot)
 * - Type-safe query key references
 */

type GameListFilters = Readonly<{
  state?: string
  viewerIsMember?: boolean
}>

function normalizeGameListFilters(filters?: GameListFilters) {
  if (!filters) return undefined

  const normalized: GameListFilters = {
    ...(filters.state ? { state: filters.state } : {}),
    ...(filters.viewerIsMember !== undefined
      ? { viewerIsMember: filters.viewerIsMember }
      : {}),
  }

  return Object.keys(normalized).length ? normalized : undefined
}

export const queryKeys = {
  games: {
    all: ['games'] as const,

    listRoot: () => ['games', 'list'] as const,
    list: (filters?: GameListFilters) => {
      const normalized = normalizeGameListFilters(filters)
      return normalized
        ? ([...queryKeys.games.listRoot(), normalized] as const)
        : ([...queryKeys.games.listRoot()] as const)
    },

    detailRoot: () => ['games', 'detail'] as const,
    detail: (id: number) => [...queryKeys.games.detailRoot(), id] as const,

    snapshot: (id: number) =>
      [...queryKeys.games.detail(id), 'snapshot'] as const,
    history: (id: number) =>
      [...queryKeys.games.detail(id), 'history'] as const,

    waitingLongest: (excludeId?: number) =>
      excludeId === undefined
        ? (['games', 'waitingLongest'] as const)
        : (['games', 'waitingLongest', { excludeId }] as const),
  },

  user: {
    all: ['user'] as const,
    options: () => ['user', 'options'] as const,
  },

  ai: {
    all: ['ai'] as const,
    registry: () => ['ai', 'registry'] as const,
  },
} as const
