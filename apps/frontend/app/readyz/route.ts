import { NextResponse } from 'next/server'

import { probeBackendReadiness } from '@/lib/server/backend-health'
import { markBackendUp, markBackendDown } from '@/lib/server/backend-status'

export async function GET(request: Request) {
  const sameOrigin =
    typeof request.url === 'string' ? new URL(request.url).origin : undefined
  const result = await probeBackendReadiness(sameOrigin)

  // Keep the server-side readiness state machine in sync with probe results.
  // Without this, mode stays 'recovering' forever after the backend comes back
  // up because probeBackendReadiness intentionally avoids touching state, causing
  // the ws-token route to keep returning 503 even after the client has recovered.
  if (result.ready) {
    markBackendUp()
  } else {
    markBackendDown(result.error)
  }

  const body = result.ready
    ? { status: 'ready', ready: true }
    : { status: 'not_ready', ready: false }

  return NextResponse.json(body, {
    status: result.ready ? 200 : 503,
    headers: { 'Cache-Control': 'no-store' },
  })
}
