/**
 * Mock WebSocket implementation for tests.
 * Supports both on* event handlers and addEventListener/removeEventListener (for MSW compatibility).
 */
export class MockWebSocket {
  static CONNECTING = 0
  static OPEN = 1
  static CLOSING = 2
  static CLOSED = 3

  readyState = MockWebSocket.CONNECTING
  url: string
  onopen: ((event: Event) => void) | null = null
  onerror: ((event: Event) => void) | null = null
  onclose: ((event: CloseEvent) => void) | null = null
  onmessage: ((event: MessageEvent) => void) | null = null

  sent: string[] = []

  // Event listeners for MSW compatibility
  private listeners: Map<string, Set<EventListener>> = new Map()

  constructor(url: string) {
    this.url = url
    // Track instance in global array
    mockWebSocketInstances.push(this)
    // Simulate async connection
    Promise.resolve().then(() => {
      this.readyState = MockWebSocket.OPEN
      this.onopen?.(new Event('open'))
      // Also trigger event listeners
      const openListeners = this.listeners.get('open')
      if (openListeners) {
        openListeners.forEach((listener) => {
          if (typeof listener === 'function') {
            listener(new Event('open'))
          }
        })
      }
    })
  }

  send(data: string) {
    this.sent.push(data)
  }

  close(code = 1000, reason = 'Connection closed') {
    this.readyState = MockWebSocket.CLOSED
    const closeEvent = new CloseEvent('close', { code, reason })
    this.onclose?.(closeEvent)
    // Also trigger event listeners
    const closeListeners = this.listeners.get('close')
    if (closeListeners) {
      closeListeners.forEach((listener) => {
        if (typeof listener === 'function') {
          listener(closeEvent)
        }
      })
    }
  }

  // MSW compatibility methods
  addEventListener(
    type: string,
    listener: EventListener | null,
    _options?: boolean | AddEventListenerOptions
  ) {
    if (!listener) return
    if (!this.listeners.has(type)) {
      this.listeners.set(type, new Set())
    }
    this.listeners.get(type)!.add(listener)
  }

  removeEventListener(
    type: string,
    listener: EventListener | null,
    _options?: boolean | EventListenerOptions
  ) {
    if (!listener) return
    this.listeners.get(type)?.delete(listener)
  }
}

/**
 * Global array to track all MockWebSocket instances for test control.
 * Should be reset in beforeEach hooks.
 */
export const mockWebSocketInstances: MockWebSocket[] = []
