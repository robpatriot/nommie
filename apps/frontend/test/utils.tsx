import React, { type ReactElement } from 'react'
import {
  render as rtlRender,
  renderHook as rtlRenderHook,
  type RenderOptions,
  type RenderResult,
  type RenderHookOptions,
  type RenderHookResult,
} from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { WebSocketProvider } from '@/lib/providers/web-socket-provider'

// AllProviders wrapper - includes QueryClientProvider + WebSocketProvider
const AllProviders = ({
  children,
  queryClient,
  isAuthenticated = false,
}: {
  children: React.ReactNode
  queryClient: QueryClient
  isAuthenticated?: boolean
}) => {
  return (
    <QueryClientProvider client={queryClient}>
      <WebSocketProvider isAuthenticated={isAuthenticated}>
        {children}
      </WebSocketProvider>
    </QueryClientProvider>
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
        gcTime: Infinity,
      },
      mutations: {
        retry: false,
      },
    },
  })
}

/**
 * Custom render function that wraps RTL's render and provides QueryClient + WebSocketProvider.
 * If a QueryClient is provided, it will be used; otherwise a new one is created.
 * Returns both the render result and the QueryClient instance.
 */
const render = (
  ui: ReactElement,
  options?: Omit<RenderOptions, 'wrapper'> & {
    queryClient?: QueryClient
    isAuthenticated?: boolean
  }
): RenderResultWithClient => {
  const queryClient = options?.queryClient ?? createTestQueryClient()
  const isAuthenticated = options?.isAuthenticated ?? true

  const {
    queryClient: _,
    isAuthenticated: __,
    ...renderOptions
  } = options ?? {}

  const renderResult = rtlRender(ui, {
    wrapper: ({ children }) => (
      <AllProviders queryClient={queryClient} isAuthenticated={isAuthenticated}>
        {children}
      </AllProviders>
    ),
    ...renderOptions,
  })

  return {
    ...renderResult,
    queryClient,
  }
}

// Extended renderHook result that includes the QueryClient
export type RenderHookResultWithClient<Result, Props> = RenderHookResult<
  Result,
  Props
> & {
  queryClient: QueryClient
}

/**
 * Custom renderHook function that wraps RTL's renderHook and provides QueryClient + WebSocketProvider.
 * If a QueryClient is provided, it will be used; otherwise a new one is created.
 * Returns both the renderHook result and the QueryClient instance.
 */
export function renderHook<Result, Props>(
  callback: (initialProps: Props) => Result,
  options?: Omit<RenderHookOptions<Props>, 'wrapper'> & {
    queryClient?: QueryClient
    isAuthenticated?: boolean
  }
): RenderHookResultWithClient<Result, Props> {
  const queryClient = options?.queryClient ?? createTestQueryClient()
  const isAuthenticated = options?.isAuthenticated ?? true

  const { queryClient: _, isAuthenticated: __, ...hookOptions } = options ?? {}

  const hookResult = rtlRenderHook(callback, {
    wrapper: ({ children }) => (
      <AllProviders queryClient={queryClient} isAuthenticated={isAuthenticated}>
        {children}
      </AllProviders>
    ),
    ...hookOptions,
  })

  return {
    ...hookResult,
    queryClient,
  }
}

// Re-export commonly used testing utilities
export * from '@testing-library/react'
export { screen } from '@testing-library/react'
export { default as userEvent } from '@testing-library/user-event'
export { render }
