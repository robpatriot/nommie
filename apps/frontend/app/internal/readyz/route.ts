import { NextResponse } from 'next/server'

import { checkBackendReadiness } from '@/lib/server/backend-health'
import { getBackendMode, getBackendStatus } from '@/lib/server/backend-status'

export async function GET() {
  const result = await checkBackendReadiness()
  const mode = getBackendMode()
  const uptimeMs = process.uptime() * 1000
  const status = getBackendStatus()

  const body = {
    service: 'frontend',
    uptime_seconds: Math.floor(uptimeMs / 1000),
    state: {
      mode,
      ready: result.ready,
    },
    dependencies: [
      {
        name: 'backend',
        status: result.ready ? 'ok' : 'down',
        checked_at: new Date().toISOString(),
        last_ok: status.lastOk ? new Date(status.lastOk).toISOString() : null,
        last_error: status.lastError ?? null,
        consecutive_successes: status.consecutiveSuccesses,
        consecutive_failures: status.consecutiveFailures,
      },
    ],
  }

  return NextResponse.json(body, {
    status: result.ready ? 200 : 503,
    headers: { 'Cache-Control': 'no-store' },
  })
}
