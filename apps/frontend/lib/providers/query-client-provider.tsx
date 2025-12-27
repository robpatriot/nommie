'use client'

import {
  QueryClient,
  QueryClientProvider,
  MutationCache,
  QueryCache,
} from '@tanstack/react-query'
import { ReactQueryDevtools } from '@tanstack/react-query-devtools'
import { useState } from 'react'
import { BackendApiError } from '@/lib/errors'
import { isNetworkError } from '@/lib/retry'
import {
  clearBackendSessionClient,
  redirectToHomeClient,
} from '@/lib/auth/clear-session-client'
import { logError, logBackendError } from '@/lib/logging/error-logger'

/**
 * Helper to check if a 5xx error is transient (should be retried).
 * Transient 5xx errors: 502 (Bad Gateway), 503 (Service Unavailable), 504 (Gateway Timeout)
 */
function isTransient5xx(status: number): boolean {
  return status === 502 || status === 503 || status === 504
}

/**
 * QueryClientProvider wrapper with default configuration.
 * Provides query client to the entire app.
 *
 * Retry Policy:
 * - Never retry 4xx responses (client errors)
 * - Retry transient 5xx responses (502/503/504) up to 1 time
 * - Retry network errors (connection failures, DNS, timeouts) up to 1 time
 * - Do not retry cancelled/aborted requests
 * - Mutations: no automatic retries (retry: 0)
 */
export function AppQueryClientProvider({
  children,
}: {
  children: React.ReactNode
}) {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        mutationCache: new MutationCache({
          onError: (error, _variables, _context, mutation) => {
            // Handle session clearing for stale user sessions
            if (
              error instanceof BackendApiError &&
              error.status === 401 &&
              error.code === 'FORBIDDEN_USER_NOT_FOUND'
            ) {
              clearBackendSessionClient()
              redirectToHomeClient()
              return // Don't log handled errors
            }

            // Log all other errors for debugging and observability
            const mutationKey =
              mutation && typeof mutation === 'object' && 'options' in mutation
                ? (mutation.options as { mutationKey?: unknown }).mutationKey
                : undefined

            if (error instanceof BackendApiError) {
              logBackendError('Mutation error in TanStack Query', error, {
                action: 'mutation',
                mutationKey,
              })
            } else {
              logError('Mutation error in TanStack Query', error, {
                action: 'mutation',
                mutationKey,
              })
            }
          },
        }),
        queryCache: new QueryCache({
          onError: (error, query) => {
            // Handle session clearing for stale user sessions
            if (
              error instanceof BackendApiError &&
              error.status === 401 &&
              error.code === 'FORBIDDEN_USER_NOT_FOUND'
            ) {
              clearBackendSessionClient()
              redirectToHomeClient()
              return // Don't log handled errors
            }

            // Log all other errors for debugging and observability
            const queryKey =
              query && typeof query === 'object' && 'queryKey' in query
                ? (query as { queryKey: unknown }).queryKey
                : undefined

            if (error instanceof BackendApiError) {
              logBackendError('Query error in TanStack Query', error, {
                action: 'query',
                queryKey,
              })
            } else {
              logError('Query error in TanStack Query', error, {
                action: 'query',
                queryKey,
              })
            }
          },
        }),
        defaultOptions: {
          queries: {
            // Stale time: how long data is considered fresh
            // 30 seconds - data won't refetch automatically for 30s
            staleTime: 30 * 1000,
            // Cache time: how long unused data stays in cache
            // 5 minutes - unused queries stay cached for 5min
            gcTime: 5 * 60 * 1000,
            // Smart retry: only retry network errors and transient 5xx
            retry: (failureCount, error) => {
              // Don't retry cancelled requests
              if (error instanceof Error && error.name === 'AbortError') {
                return false
              }

              // Handle BackendApiError (application errors with status codes)
              if (error instanceof BackendApiError) {
                const status = error.status

                // Never retry 4xx (client errors)
                if (status >= 400 && status < 500) {
                  return false
                }

                // Retry transient 5xx errors (502, 503, 504) once
                if (isTransient5xx(status)) {
                  return failureCount < 1
                }

                // Don't retry other 5xx (could be application bugs we don't want to mask)
                return false
              }

              // Retry genuine network errors (connection failures, DNS, timeouts)
              if (isNetworkError(error)) {
                return failureCount < 1
              }

              // Don't retry unknown error types
              return false
            },
            // Exponential backoff for retries: 500ms, 1000ms (capped at 2000ms)
            retryDelay: (attemptIndex) => {
              const baseDelay = 500
              const maxDelay = 2000
              const delay = Math.min(
                baseDelay * Math.pow(2, attemptIndex),
                maxDelay
              )
              return delay
            },
            // Refetch on window focus (good for keeping data fresh)
            refetchOnWindowFocus: true,
            // Refetch on reconnect (auto-heal after connectivity returns)
            refetchOnReconnect: true,
          },
          mutations: {
            // Retry failed mutations 0 times (mutations shouldn't retry automatically)
            retry: 0,
          },
        },
      })
  )

  return (
    <QueryClientProvider client={queryClient}>
      {children}
      {process.env.NODE_ENV === 'development' && (
        <ReactQueryDevtools initialIsOpen={false} />
      )}
    </QueryClientProvider>
  )
}
