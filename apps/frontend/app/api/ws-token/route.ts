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
  isBackendKnownDown,
} from '@/lib/server/backend-status'
import { isBackendConnectionError } from '@/lib/server/connection-errors'
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
    markBackendUp('ws_token_route')

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
      // When the backend is known unavailable the JWT refresh failed due to
      // infrastructure, not because the session is stale. Return 503 so the
      // client enters degraded mode instead of redirecting to signout.
      if (isBackendKnownDown()) {
        return NextResponse.json(
          { error: 'Backend not ready' },
          { status: 503 }
        )
      }
      return NextResponse.json(
        { error: 'Authentication required' },
        { status: 401 }
      )
    }

    const isConnectionErr = isBackendConnectionError(error)

    if (isConnectionErr) {
      const msg = error instanceof Error ? error.message : String(error)
      markBackendDown(msg, 'ws_token_route')
      // Always return 503 for connection errors regardless of current mode.
      // This stops the WS provider from scheduling a reconnect (which would
      // just generate another connection error) and ensures triggerRecovery()
      // is called on the client side exactly once.
      return NextResponse.json({ error: 'Backend not ready' }, { status: 503 })
    }

    // Non-connection errors are genuine application failures — log and return 500.
    if (shouldLogError()) {
      logError('Unexpected websocket token error', error, {
        action: 'fetchWsToken',
      })
    }

    return NextResponse.json(
      { error: 'Unable to issue websocket token' },
      { status: 500 }
    )
  }
}
