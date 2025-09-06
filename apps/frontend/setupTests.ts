import '@testing-library/jest-dom/vitest'
import { vi } from 'vitest'

// Import mocks to register them globally
import './test/mocks/next-auth'
import './test/mocks/next-navigation'

// Clean up after each test
afterEach(() => {
  vi.restoreAllMocks()
  vi.clearAllMocks()
})
