import { vi } from 'vitest'

// Mock user type - adjust based on your actual User type
export interface MockUser {
  id: string
  name?: string | null
  email?: string | null
  image?: string | null
}

// Default mock user
const defaultUser: MockUser = {
  id: '1',
  name: 'Test User',
  email: 'test@example.com',
  image: 'https://example.com/avatar.jpg',
}

// Session state
let mockSession: { user: MockUser; expires: string } | null = null
let mockStatus: 'loading' | 'authenticated' | 'unauthenticated' =
  'unauthenticated'

// Mock functions
export const mockSignIn = vi.fn()
export const mockSignOut = vi.fn()

// Helper functions to control session state
export const mockUnauthenticated = () => {
  mockSession = null
  mockStatus = 'unauthenticated'
}

export const mockAuthenticated = (user?: Partial<MockUser>) => {
  mockSession = {
    user: { ...defaultUser, ...user },
    expires: new Date(Date.now() + 30 * 24 * 60 * 60 * 1000).toISOString(), // 30 days from now
  }
  mockStatus = 'authenticated'
}

export const mockLoading = () => {
  mockSession = null
  mockStatus = 'loading'
}

// Mock the next-auth/react module
vi.mock('next-auth/react', () => ({
  useSession: () => ({
    data: mockSession,
    status: mockStatus,
  }),
  signIn: mockSignIn,
  signOut: mockSignOut,
  SessionProvider: ({ children }: { children: React.ReactNode }) => children,
}))

// Reset function for cleanup
export const resetAuthMocks = () => {
  mockUnauthenticated()
  mockSignIn.mockClear()
  mockSignOut.mockClear()
}
