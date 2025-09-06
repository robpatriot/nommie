import '@testing-library/jest-dom/vitest'
import { vi } from 'vitest'

// Import mocks to register them globally
import './test/mocks/next-auth'
import './test/mocks/next-navigation'

// Import MSW server
import { server } from './test/msw/server'

// MSW server lifecycle hooks
beforeAll(() => {
  server.listen({ onUnhandledRequest: 'error' })
})

afterEach(() => {
  // Reset MSW handlers to clean slate
  server.resetHandlers()
  // Clean up Vitest mocks
  vi.restoreAllMocks()
  vi.clearAllMocks()
})

afterAll(() => {
  server.close()
})
