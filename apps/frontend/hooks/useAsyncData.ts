'use client'

import { useCallback, useEffect, useRef, useState } from 'react'

/**
 * Options for configuring useAsyncData behavior
 */
export interface UseAsyncDataOptions<T> {
  /**
   * Whether to automatically fetch when enabled becomes true or dependencies change
   * @default false
   */
  autoFetch?: boolean

  /**
   * Whether fetching is currently enabled
   * Only used when autoFetch is true
   * @default true
   */
  enabled?: boolean

  /**
   * Optional transform function to map the fetched data before storing it
   */
  transform?: (data: T) => T

  /**
   * Optional function to reset state when dependencies change
   */
  onReset?: () => void
}

/**
 * Reusable hook for async data fetching with loading, error, and cancellation support.
 * Supports both manual fetching and automatic fetching based on dependencies.
 *
 * @param fetchFn - Async function that fetches the data
 * @param dependencies - Array of dependencies that trigger auto-fetch (when autoFetch is true)
 * @param options - Configuration options
 * @returns Object containing data, error, isLoading, fetchData, and reset functions
 *
 * @example
 * // Manual fetching
 * const { data, error, isLoading, fetchData } = useAsyncData(
 *   async () => await fetchSomething(),
 *   []
 * )
 *
 * @example
 * // Auto-fetch on dependency change
 * const { data, error, isLoading } = useAsyncData(
 *   async () => await fetchSomething(id),
 *   [id],
 *   { autoFetch: true, enabled: !!id }
 * )
 */
export function useAsyncData<T>(
  fetchFn: () => Promise<T>,
  dependencies: unknown[] = [],
  options: UseAsyncDataOptions<T> = {}
): {
  data: T | null
  error: string | null
  isLoading: boolean
  fetchData: () => Promise<T | null>
  reset: () => void
} {
  const { autoFetch = false, enabled = true, transform, onReset } = options

  const [data, setData] = useState<T | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const cancelledRef = useRef(false)

  // Reset function to clear all state
  const reset = useCallback(() => {
    cancelledRef.current = true
    setData(null)
    setError(null)
    setIsLoading(false)
    onReset?.()
  }, [onReset])

  // Reset when dependencies change (for auto-fetch mode)
  useEffect(() => {
    if (autoFetch) {
      reset()
      cancelledRef.current = false
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, dependencies)

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      cancelledRef.current = true
    }
  }, [])

  // Manual fetch function
  const fetchData = useCallback(async (): Promise<T | null> => {
    cancelledRef.current = false
    setIsLoading(true)
    setError(null)

    try {
      const result = await fetchFn()
      if (cancelledRef.current) {
        return null
      }

      const transformedData = transform ? transform(result) : result
      setData(transformedData)
      return transformedData
    } catch (err) {
      if (cancelledRef.current) {
        return null
      }

      const message =
        err instanceof Error ? err.message : 'Failed to fetch data'
      setError(message)
      return null
    } finally {
      if (!cancelledRef.current) {
        setIsLoading(false)
      }
    }
  }, [fetchFn, transform])

  // Memoize the fetch function to prevent infinite loops
  const memoizedFetchFn = useCallback(() => fetchFn(), [fetchFn])

  // Auto-fetch effect
  useEffect(() => {
    if (!autoFetch) {
      return
    }

    // Reset state when disabled
    if (!enabled) {
      cancelledRef.current = true
      setData(null)
      setError(null)
      setIsLoading(false)
      onReset?.()
      return
    }

    cancelledRef.current = false
    setIsLoading(true)
    setError(null)

    void memoizedFetchFn()
      .then((result) => {
        if (cancelledRef.current) {
          return
        }

        const transformedData = transform ? transform(result) : result
        setData(transformedData)
      })
      .catch((err) => {
        if (cancelledRef.current) {
          return
        }

        const message =
          err instanceof Error ? err.message : 'Failed to fetch data'
        setError(message)
      })
      .finally(() => {
        if (!cancelledRef.current) {
          setIsLoading(false)
        }
      })

    return () => {
      cancelledRef.current = true
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [autoFetch, enabled, memoizedFetchFn, transform, onReset, ...dependencies])

  return { data, error, isLoading, fetchData, reset }
}
