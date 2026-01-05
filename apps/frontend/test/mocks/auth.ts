import { vi } from 'vitest'

// Mock @/auth to prevent next-auth from being loaded in tests
// This prevents the "Cannot find module 'next/server'" error
vi.mock('@/auth', () => ({
  auth: vi.fn(() => Promise.resolve(null)),
  signIn: vi.fn(),
  signOut: vi.fn(),
  handlers: {
    GET: vi.fn(),
    POST: vi.fn(),
  },
  unstable_update: vi.fn(),
  BACKEND_BASE_URL_ERROR_MSG:
    'NEXT_PUBLIC_BACKEND_BASE_URL must be set to an absolute URL when minting backend JWT',
  getBackendBaseUrlOrThrow: vi.fn(() => 'http://localhost:3001'),
}))
