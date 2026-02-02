import '@testing-library/jest-dom/vitest'
import { cleanup, configure } from '@testing-library/react'
import { act } from 'react'
import { beforeAll, afterAll, afterEach, vi } from 'vitest'

import { server } from './test/msw/server'

// Import mocks to register them globally
import './test/mocks/next-auth'
import './test/mocks/next-navigation'
import './test/mocks/next-server'
import './test/mocks/next-intl'
import './test/mocks/auth'

import { MockWebSocket } from '@/test/setup/mock-websocket'

// Make @testing-library/react waitFor cooperate with fake timers
configure({
  unstable_advanceTimersWrapper: async (cb) => {
    await act(async () => {
      await cb() // Ensure the callback completes
    })
  },
})

// Global WebSocket stub
vi.stubGlobal('WebSocket', MockWebSocket)

// Default WebSocket config for tests.
// Many components mount WebSocketProvider via the shared test utils; provide a valid URL
// so config validation doesn't fail (WebSocket itself is mocked).
if (
  !process.env.NEXT_PUBLIC_BACKEND_WS_URL &&
  !process.env.NEXT_PUBLIC_BACKEND_BASE_URL
) {
  process.env.NEXT_PUBLIC_BACKEND_WS_URL = 'ws://localhost:3001'
}

// matchMedia mock
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

// ResizeObserver mock
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

// MSW lifecycle hooks
beforeAll(() => {
  server.listen({
    onUnhandledRequest(request, print) {
      const url = request.url
      // We mock WebSockets via MockWebSocket in tests, not MSW.
      // MSW can still detect WS connections and warn if there is no ws.link handler.
      if (url.startsWith('ws://') || url.startsWith('wss://')) {
        return
      }
      print.warning()
    },
  })
})

afterEach(() => {
  cleanup()
  server.resetHandlers()
  vi.clearAllMocks()
})

afterAll(() => {
  server.close()
})
