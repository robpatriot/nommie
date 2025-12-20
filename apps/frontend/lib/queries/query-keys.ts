/**
 * Centralized query key factory for TanStack Query.
 * All query keys should be generated through these factories.
 *
 * Query keys are hierarchical arrays that allow for:
 * - Invalidating related queries (e.g., invalidate all games queries)
 * - Precise cache invalidation (e.g., invalidate specific game snapshot)
 * - Type-safe query key references
 */

export const queryKeys = {
  // Game queries
  games: {
    all: ['games'] as const,
    lists: () => [...queryKeys.games.all, 'list'] as const,
    list: (filters?: { state?: string; viewerIsMember?: boolean }) => {
      // Normalize filters: only include in key if filters exist and have values
      // This ensures consistent keys regardless of how filters are passed
      if (!filters || Object.keys(filters).length === 0) {
        return [...queryKeys.games.lists()] as const
      }
      return [...queryKeys.games.lists(), filters] as const
    },
    details: () => [...queryKeys.games.all, 'detail'] as const,
    detail: (id: number) => [...queryKeys.games.details(), id] as const,
    snapshot: (id: number) =>
      [...queryKeys.games.detail(id), 'snapshot'] as const,
    history: (id: number) =>
      [...queryKeys.games.detail(id), 'history'] as const,
    lastActive: () => [...queryKeys.games.all, 'lastActive'] as const,
  },

  // User queries
  user: {
    all: ['user'] as const,
    options: () => [...queryKeys.user.all, 'options'] as const,
  },

  // AI registry queries
  ai: {
    all: ['ai'] as const,
    registry: () => [...queryKeys.ai.all, 'registry'] as const,
  },
} as const
