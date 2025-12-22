import {
  render as rtlRender,
  type RenderOptions,
  type RenderResult,
} from '@testing-library/react'
import type { ReactElement } from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'

// AllProviders wrapper - includes QueryClientProvider for TanStack Query
const AllProviders = ({
  children,
  queryClient,
}: {
  children: React.ReactNode
  queryClient: QueryClient
}) => {
  return (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  )
}

// Extended render result that includes the QueryClient
export interface RenderResultWithClient extends RenderResult {
  queryClient: QueryClient
}

/**
 * Creates a new QueryClient for tests with appropriate default options.
 * Each test should create its own QueryClient to ensure isolation.
 */
export function createTestQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: 0,
      },
      mutations: {
        retry: false,
      },
    },
  })
}

/**
 * Custom render function that wraps RTL's render and provides QueryClient.
 * If a QueryClient is provided, it will be used; otherwise a new one is created.
 * Returns both the render result and the QueryClient instance.
 */
const render = (
  ui: ReactElement,
  options?: Omit<RenderOptions, 'wrapper'> & {
    queryClient?: QueryClient
  }
): RenderResultWithClient => {
  // Use provided QueryClient or create a new one for test isolation
  const queryClient = options?.queryClient ?? createTestQueryClient()

  const { queryClient: _, ...renderOptions } = options ?? {}

  const renderResult = rtlRender(ui, {
    wrapper: ({ children }) => (
      <AllProviders queryClient={queryClient}>{children}</AllProviders>
    ),
    ...renderOptions,
  })

  return {
    ...renderResult,
    queryClient,
  }
}

// Re-export commonly used testing utilities
export * from '@testing-library/react'
export { screen } from '@testing-library/react'
export { default as userEvent } from '@testing-library/user-event'
export { render }
