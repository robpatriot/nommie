'use client'

import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { ReactQueryDevtools } from '@tanstack/react-query-devtools'
import { useState } from 'react'

/**
 * QueryClientProvider wrapper with default configuration.
 * Provides query client to the entire app.
 */
export function AppQueryClientProvider({
  children,
}: {
  children: React.ReactNode
}) {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            // Stale time: how long data is considered fresh
            // 30 seconds - data won't refetch automatically for 30s
            staleTime: 30 * 1000,
            // Cache time: how long unused data stays in cache
            // 5 minutes - unused queries stay cached for 5min
            gcTime: 5 * 60 * 1000,
            // Retry failed requests 1 time
            retry: 1,
            // Refetch on window focus (good for keeping data fresh)
            refetchOnWindowFocus: true,
            // Don't refetch on reconnect by default (can be overridden per query)
            refetchOnReconnect: false,
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
