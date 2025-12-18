import '@testing-library/jest-dom/vitest'
import { cleanup } from '@testing-library/react'
import { beforeAll, afterAll, afterEach, vi } from 'vitest'

// Import mocks to register them globally
import './test/mocks/next-auth'
import './test/mocks/next-navigation'
import './test/mocks/next-server'
import './test/mocks/auth'

// Import MSW server
import { server } from './test/msw/server'

if (typeof window !== 'undefined' && typeof window.matchMedia !== 'function') {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: (query: string) => {
      const minWidthMatch = query.match(/min-width:\s*(\d+)px/i)
      const maxWidthMatch = query.match(/max-width:\s*(\d+)px/i)

      const minWidth = minWidthMatch ? Number(minWidthMatch[1]) : null
      const maxWidth = maxWidthMatch ? Number(maxWidthMatch[1]) : null

      const viewportWidth =
        typeof window !== 'undefined' && typeof window.innerWidth === 'number'
          ? window.innerWidth
          : 1024

      const matches =
        (minWidth === null || viewportWidth >= minWidth) &&
        (maxWidth === null || viewportWidth <= maxWidth) &&
        (minWidth !== null || maxWidth !== null)

      return {
        matches,
        media: query,
        onchange: null,
        addListener: vi.fn(),
        removeListener: vi.fn(),
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        dispatchEvent: vi.fn(),
      }
    },
  })
}

if (typeof globalThis.ResizeObserver === 'undefined') {
  class ResizeObserverMock {
    callback: ResizeObserverCallback
    observe = vi.fn()
    unobserve = vi.fn()
    disconnect = vi.fn()

    constructor(callback: ResizeObserverCallback) {
      this.callback = callback
    }
  }

  globalThis.ResizeObserver =
    ResizeObserverMock as unknown as typeof ResizeObserver
}

// MSW server lifecycle hooks
beforeAll(() => {
  server.listen({ onUnhandledRequest: 'error' })
})

afterEach(() => {
  cleanup()
  // Reset MSW handlers to clean slate
  server.resetHandlers()
  // Clean up Vitest mocks
  vi.restoreAllMocks()
  vi.clearAllMocks()
})

afterAll(() => {
  server.close()
})
