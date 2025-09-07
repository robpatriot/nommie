import { vi } from 'vitest'

// Mock router state
let mockPathname = '/'
let mockSearchParams = new URLSearchParams()
let mockRouter = {
  push: vi.fn(),
  replace: vi.fn(),
  back: vi.fn(),
  forward: vi.fn(),
  refresh: vi.fn(),
  prefetch: vi.fn(),
}

// Helper functions to control router state
export const mockUseRouter = (routerOverrides?: Partial<typeof mockRouter>) => {
  Object.assign(mockRouter, routerOverrides)
  return mockRouter
}

export const mockUsePathname = (pathname: string) => {
  mockPathname = pathname
}

export const mockUseSearchParams = (
  searchParams: Record<string, string> | URLSearchParams
) => {
  mockSearchParams = new URLSearchParams(searchParams)
}

// Mock the next/navigation module
vi.mock('next/navigation', () => ({
  useRouter: () => mockRouter,
  usePathname: () => mockPathname,
  useSearchParams: () => mockSearchParams,
  useParams: () => ({}),
  notFound: vi.fn(),
  redirect: vi.fn(),
  permanentRedirect: vi.fn(),
}))

// Reset function for cleanup
export const resetNavigationMocks = () => {
  mockPathname = '/'
  mockSearchParams = new URLSearchParams()
  mockRouter = {
    push: vi.fn(),
    replace: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
    refresh: vi.fn(),
    prefetch: vi.fn(),
  }
}
