/**
 * Environment variable validation.
 * Validates required environment variables at app startup to provide
 * clear error messages before runtime failures.
 */

/**
 * Validates WebSocket URL environment variables.
 * Throws an error with a clear message if configuration is missing.
 * Called when establishing WebSocket connections to provide clear error messages.
 */
export function validateWebSocketConfig(): void {
  const explicitBase = process.env.NEXT_PUBLIC_BACKEND_WS_URL
  const httpBase = process.env.NEXT_PUBLIC_BACKEND_BASE_URL

  if (!explicitBase && !httpBase) {
    throw new Error(
      'WebSocket configuration missing: NEXT_PUBLIC_BACKEND_WS_URL or NEXT_PUBLIC_BACKEND_BASE_URL must be configured'
    )
  }

  // Validate URL format if provided
  if (explicitBase) {
    try {
      // Remove trailing slash and validate
      const url = explicitBase.replace(/\/$/, '')
      new URL(url) // This will throw if invalid
    } catch {
      throw new Error(
        `Invalid NEXT_PUBLIC_BACKEND_WS_URL format: ${explicitBase}`
      )
    }
  }

  if (httpBase) {
    try {
      const url = httpBase.replace(/\/$/, '')
      new URL(url) // This will throw if invalid
    } catch {
      throw new Error(
        `Invalid NEXT_PUBLIC_BACKEND_BASE_URL format: ${httpBase}`
      )
    }
  }
}

/**
 * Resolves WebSocket URL from environment variables.
 * Assumes validation has already been performed (use validateWebSocketConfig).
 * @returns The WebSocket URL (ws:// or wss://)
 */
export function resolveWebSocketUrl(): string {
  const explicitBase = process.env.NEXT_PUBLIC_BACKEND_WS_URL
  if (explicitBase) {
    return explicitBase.replace(/\/$/, '')
  }

  const httpBase = process.env.NEXT_PUBLIC_BACKEND_BASE_URL
  if (httpBase) {
    // Convert http:// to ws:// and https:// to wss://
    return httpBase
      .replace(/\/$/, '')
      .replace(/^https?/, (match) => (match === 'https' ? 'wss' : 'ws'))
  }

  // This should never happen if validateWebSocketConfig was called
  throw new Error(
    'WebSocket URL resolution failed: NEXT_PUBLIC_BACKEND_WS_URL or NEXT_PUBLIC_BACKEND_BASE_URL must be configured'
  )
}
