import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, waitFor, act, createTestQueryClient } from '../utils'
import type { QueryClient } from '@tanstack/react-query'
import { useWebSocket } from '@/lib/providers/web-socket-provider'
import { MockWebSocket, mockWebSocketInstances } from '../setup/mock-websocket'
import { setupFetchMock } from '../setup/game-room-client-mocks'

// Track original fetch
const originalFetch = globalThis.fetch

// Global WebSocket mock
vi.stubGlobal('WebSocket', MockWebSocket)

// Mock WebSocket config validation
vi.mock('@/lib/config/env-validation', () => ({
  resolveWebSocketUrl: () => 'ws://localhost:3001',
  validateWebSocketConfig: () => {},
}))

// Mock error logger to avoid console noise
vi.mock('@/lib/logging/error-logger', () => ({
  logError: vi.fn(),
}))

function serverSendJson(ws: MockWebSocket, msg: unknown) {
  ws.onmessage?.(
    new MessageEvent('message', {
      data: JSON.stringify(msg),
    })
  )
}

function getSentJson(ws: MockWebSocket): unknown[] {
  return ws.sent.map((s) => {
    try {
      return JSON.parse(s) as unknown
    } catch {
      return s
    }
  })
}

function findSentByType<T extends { type: string }>(
  ws: MockWebSocket,
  type: string
): T | undefined {
  return getSentJson(ws).find(
    (m): m is T =>
      typeof m === 'object' && m !== null && (m as any).type === type
  )
}

async function waitForWsCount(n: number) {
  await waitFor(() => {
    expect(mockWebSocketInstances.length).toBe(n)
  })
}

async function waitForSentType(ws: MockWebSocket, type: string) {
  await waitFor(() => {
    expect(findSentByType(ws, type)).toBeDefined()
  })
}

describe('WebSocketProvider', () => {
  let queryClient: QueryClient

  beforeEach(() => {
    queryClient = createTestQueryClient()

    mockWebSocketInstances.length = 0
    vi.clearAllMocks()
    vi.useRealTimers()
    setupFetchMock(originalFetch)
    vi.stubGlobal('WebSocket', MockWebSocket)
    process.env.NEXT_PUBLIC_BACKEND_BASE_URL = 'http://localhost:3001'
  })

  afterEach(() => {
    mockWebSocketInstances.forEach((ws) => {
      ws.onopen = null
      ws.onmessage = null
      ws.onerror = null
      ws.onclose = null
    })
    mockWebSocketInstances.length = 0
    vi.clearAllMocks()
    vi.useRealTimers()
  })

  it('should connect on mount when authenticated', async () => {
    const { result } = renderHook(() => useWebSocket(), {
      queryClient,
      isAuthenticated: true,
    })

    await waitForWsCount(1)
    const ws = mockWebSocketInstances[0]

    // connect() happens in a useEffect, so don't assume synchronous state.
    await waitFor(() => {
      expect(result.current.connectionState).toBe('connecting')
    })

    // MockWebSocket fires onopen in a microtask; wait for hello to be sent.
    await waitForSentType(ws, 'hello')

    await act(async () => {
      serverSendJson(ws, { type: 'hello_ack', protocol: 1, user_id: 123 })
    })

    await waitFor(() => {
      expect(result.current.connectionState).toBe('connected')
    })
  })

  it('does not connect when not authenticated', async () => {
    const { result } = renderHook(() => useWebSocket(), {
      queryClient,
      isAuthenticated: false,
    })

    // No socket should be created
    await waitFor(() => {
      expect(mockWebSocketInstances.length).toBe(0)
      expect(result.current.connectionState).toBe('disconnected')
    })
  })

  it('should broadcast messages to registered handlers', async () => {
    const { result } = renderHook(() => useWebSocket(), {
      queryClient,
      isAuthenticated: true,
    })

    await waitForWsCount(1)
    const ws = mockWebSocketInstances[0]

    // Complete handshake (and note: handler is not registered yet)
    await waitForSentType(ws, 'hello')
    await act(async () => {
      serverSendJson(ws, { type: 'hello_ack', protocol: 1, user_id: 123 })
    })

    const messages: unknown[] = []
    const handler = (msg: unknown) => messages.push(msg)

    let unsubscribe!: () => void
    act(() => {
      unsubscribe = result.current.registerHandler(handler)
    })

    // Must pass isWireMsg: { type: string, ... }
    const testMsg = { type: 'test_msg', data: 'hello' }
    act(() => {
      serverSendJson(ws, testMsg)
    })

    expect(messages).toContainEqual(testMsg)

    act(() => {
      unsubscribe()
    })

    act(() => {
      serverSendJson(ws, { type: 'another_msg' })
    })

    // We registered after hello_ack was processed, so we only saw test_msg.
    expect(messages).toEqual([testMsg])
  })

  it('should attempt reconnection on connection failure', async () => {
    // Use real timers (no fake timers)
    vi.useRealTimers()

    const { result } = renderHook(() => useWebSocket(), {
      queryClient,
      isAuthenticated: true,
    })

    // Wait for the first WebSocket to be initialized
    await waitForWsCount(1)
    const ws0 = mockWebSocketInstances[0]

    // Trigger reconnect path by closing the first WebSocket (ws0)
    act(() => {
      ws0.close(1006, 'abnormal closure')
    })

    // Wait for connectionState to be 'reconnecting'
    await waitFor(() => {
      expect(result.current.connectionState).toBe('reconnecting')
    })

    // Using real timers here, so the timer will work as expected
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 1000))
    })

    // Wait for WebSocket count to increase (reconnect should happen)
    await waitForWsCount(2)

    const ws1 = mockWebSocketInstances[1]

    // Wait for WebSocket "hello" message to be sent
    await waitForSentType(ws1, 'hello')

    // Simulate server acknowledgment
    await act(async () => {
      serverSendJson(ws1, { type: 'hello_ack', protocol: 1, user_id: 123 })
    })

    // Wait for connection state to be "connected"
    await waitFor(() => {
      expect(result.current.connectionState).toBe('connected')
    })
  })

  it('disconnect() closes the socket', async () => {
    const { result } = renderHook(() => useWebSocket(), {
      queryClient,
      isAuthenticated: true,
    })

    await waitForWsCount(1)
    const ws = mockWebSocketInstances[0]

    await waitForSentType(ws, 'hello')
    await act(async () => {
      serverSendJson(ws, { type: 'hello_ack', protocol: 1, user_id: 123 })
    })

    await waitFor(() => {
      expect(result.current.connectionState).toBe('connected')
    })

    const closeSpy = vi.spyOn(ws, 'close')

    act(() => {
      result.current.disconnect()
    })

    expect(closeSpy).toHaveBeenCalled()
    await waitFor(() => {
      expect(result.current.connectionState).toBe('disconnected')
    })
  })

  it('sets syncError when ws-token fetch times out', async () => {
    // Use real timers (no fake timers)
    vi.useRealTimers()

    // Make fetch never resolve so AbortController timeout triggers
    vi.stubGlobal(
      'fetch',
      vi.fn((input: string | URL | Request, init?: RequestInit) => {
        const urlString =
          typeof input === 'string'
            ? input
            : input instanceof URL
              ? input.toString()
              : input.url

        if (urlString.includes('/api/ws-token')) {
          const signal = init?.signal
          return new Promise((_, reject) => {
            const err = new Error('Aborted')
            ;(err as any).name = 'AbortError'

            // Make sure the timeout triggers
            setTimeout(() => {
              reject(err)
            }, 100) // Ensure rejection occurs after timeout

            if (signal?.aborted) return reject(err)
            signal?.addEventListener('abort', () => reject(err), { once: true })
          }) as any
        }

        return originalFetch(input as any, init as any)
      })
    )

    const { result } = renderHook(() => useWebSocket(), {
      queryClient,
      isAuthenticated: true,
    })

    // Use real timers and set the timeout to 100ms
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100))
    })

    await waitFor(() => {
      expect(result.current.syncError).toBeTruthy() // Ensure syncError is truthy
      expect(result.current.syncError?.message).toContain('timed out') // Check the timeout message
    })
  })

  it('sets syncError when ws-token fetch fails with non-401', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn((url: string | URL | Request) => {
        const urlString =
          typeof url === 'string'
            ? url
            : url instanceof URL
              ? url.toString()
              : url.url
        if (urlString.includes('/api/ws-token')) {
          return Promise.resolve({
            ok: false,
            status: 500,
            statusText: 'Internal Server Error',
          } as Response)
        }
        return originalFetch(url as any)
      })
    )

    const { result } = renderHook(() => useWebSocket(), {
      queryClient,
      isAuthenticated: true,
    })

    await waitFor(() => {
      expect(result.current.syncError).toBeTruthy()
      expect(result.current.syncError?.message).toContain('500')
    })
  })

  it('sets syncError on websocket error event', async () => {
    const { result } = renderHook(() => useWebSocket(), {
      queryClient,
      isAuthenticated: true,
    })

    await waitForWsCount(1)
    const ws = mockWebSocketInstances[0]

    // complete hello -> connected
    await waitForSentType(ws, 'hello')
    await act(async () => {
      serverSendJson(ws, { type: 'hello_ack', protocol: 1, user_id: 123 })
    })

    await waitFor(() => {
      expect(result.current.connectionState).toBe('connected')
    })

    act(() => {
      ws.onerror?.(new Event('error'))
    })

    await waitFor(() => {
      expect(result.current.syncError).toBeTruthy()
      expect(result.current.syncError?.message).toContain(
        'Websocket connection error'
      )
    })
  })
})
