import { NextResponse } from 'next/server'

import { getBackendBaseUrlOrThrow } from '@/auth'
import {
  ensureBackendJwtForServerAction,
  BackendJwtError,
} from '@/lib/auth/refresh-backend-jwt'
import {
  shouldLogError,
  markBackendUp,
  markBackendDown,
  getBackendMode,
  getBackendStatus,
} from '@/lib/server/backend-status'
import {
  isBackendStartupError,
  isBackendConnectionError,
} from '@/lib/server/connection-errors'
import { logError } from '@/lib/logging/error-logger'

export async function GET() {
  // Only short-circuit when we already know the backend is down (avoid calling it).
  // On first load we are in 'startup' with 0 failures – try the backend; once we have
  // failures or are 'recovering', return 503 without calling.
  const mode = getBackendMode()
  const { consecutiveFailures } = getBackendStatus()
  const knownDown =
    mode === 'recovering' || (mode === 'startup' && consecutiveFailures > 0)
  if (knownDown) {
    return NextResponse.json({ error: 'Backend not ready' }, { status: 503 })
  }

  try {
    const jwt = await ensureBackendJwtForServerAction()
    const backendBase = getBackendBaseUrlOrThrow()

    const response = await fetch(`${backendBase}/api/ws/token`, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${jwt}`,
        'Content-Type': 'application/json',
      },
      cache: 'no-store',
    })

    // Mark backend as up if we got a response (even if error) - connection succeeded
    // This helps differentiate startup failures from runtime failures
    markBackendUp()

    if (!response.ok) {
      // Only log if we should (outside startup window or runtime failure)
      if (shouldLogError()) {
        logError(
          'Failed to fetch websocket token',
          new Error(`Status: ${response.status}`),
          {
            action: 'fetchWsToken',
            status: response.status,
          }
        )
      }
      return NextResponse.json(
        { error: 'Unable to issue websocket token' },
        { status: response.status }
      )
    }

    const payload = await response.json()
    return NextResponse.json(payload, { status: 200 })
  } catch (error) {
    if (error instanceof BackendJwtError) {
      // Return 401 for auth errors so client can redirect
      return NextResponse.json(
        { error: 'Authentication required' },
        { status: 401 }
      )
    }

    if (isBackendConnectionError(error)) {
      const msg = error instanceof Error ? error.message : String(error)
      markBackendDown(msg)
    }

    // Check if this is a backend startup error
    const isStartupError = isBackendStartupError(error)

    // Only log if we should (outside startup window or runtime failure)
    if (shouldLogError() && !isStartupError) {
      logError('Unexpected websocket token error', error, {
        action: 'fetchWsToken',
      })
    }

    // Return 503 for startup errors (retriable), 500 for other errors
    return NextResponse.json(
      { error: 'Unable to issue websocket token' },
      { status: isStartupError ? 503 : 500 }
    )
  }
}
