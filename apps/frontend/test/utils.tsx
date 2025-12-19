import {
  render as rtlRender,
  type RenderOptions,
  type RenderResult,
} from '@testing-library/react'
import { ReactElement, useMemo, useEffect } from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'

// Store the current query client for use in mocks
let currentQueryClient: QueryClient | null = null

// AllProviders wrapper - includes QueryClientProvider for TanStack Query
const AllProviders = ({ children }: { children: React.ReactNode }) => {
  // Create a new QueryClient for each test to ensure isolation
  // Use useMemo to ensure the QueryClient is stable across renders
  const queryClient = useMemo(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            retry: false,
            gcTime: 0,
          },
          mutations: {
            retry: false,
          },
        },
      }),
    []
  )

  // Store for use in mocks - use useEffect to avoid side effects during render
  useEffect(() => {
    currentQueryClient = queryClient
  }, [queryClient])

  return (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  )
}

// Export function to get the current query client (for use in mocks)
export function getTestQueryClient(): QueryClient | null {
  return currentQueryClient
}

// Custom render function that wraps RTL's render
const render = (
  ui: ReactElement,
  options?: Omit<RenderOptions, 'wrapper'>
): RenderResult => rtlRender(ui, { wrapper: AllProviders, ...options })

// Re-export commonly used testing utilities
export * from '@testing-library/react'
export { screen } from '@testing-library/react'
export { default as userEvent } from '@testing-library/user-event'
export { render }
