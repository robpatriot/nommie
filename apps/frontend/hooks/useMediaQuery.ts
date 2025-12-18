import { useSyncExternalStore } from 'react'

/**
 * Custom hook for responsive media queries
 * Returns true when the media query matches
 *
 * This hook is hydration-safe: it always returns false during SSR and
 * the initial client render to prevent hydration mismatches. Uses
 * useSyncExternalStore to properly subscribe to media query changes.
 */
export function useMediaQuery(query: string): boolean {
  // useSyncExternalStore is the recommended way to subscribe to external stores
  // like MediaQueryList. It handles hydration safety automatically.
  return useSyncExternalStore(
    (onStoreChange) => {
      if (typeof window === 'undefined') {
        // Return a no-op subscription for SSR
        return () => {}
      }

      const mediaQuery = window.matchMedia(query)

      // Subscribe to changes - call onStoreChange when the media query changes
      mediaQuery.addEventListener('change', onStoreChange)

      // Return unsubscribe function
      return () => {
        mediaQuery.removeEventListener('change', onStoreChange)
      }
    },
    () => {
      // getSnapshot: return current value
      if (typeof window === 'undefined') {
        return false
      }
      return window.matchMedia(query).matches
    },
    () => {
      // getServerSnapshot: return value for SSR (always false to match initial client render)
      return false
    }
  )
}
