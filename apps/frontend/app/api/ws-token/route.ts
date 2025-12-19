import { NextResponse } from 'next/server'

import { getBackendBaseUrlOrThrow } from '@/auth'
import { ensureBackendJwtForServerAction } from '@/lib/auth/refresh-backend-jwt'
import {
  shouldLogError,
  isInStartupWindow,
  markBackendUp,
} from '@/lib/server/backend-status'
import { isBackendStartupError } from '@/lib/server/connection-errors'
import { logError } from '@/lib/logging/error-logger'

export async function GET() {
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
    // Check if this is a backend startup error
    const isStartupError = isBackendStartupError(error, isInStartupWindow)

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
