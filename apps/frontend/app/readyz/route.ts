import { NextResponse } from 'next/server'

import { probeBackendReadiness } from '@/lib/server/backend-health'

export async function GET(request: Request) {
  const sameOrigin =
    typeof request.url === 'string' ? new URL(request.url).origin : undefined
  const result = await probeBackendReadiness(sameOrigin)

  const body = result.ready
    ? { status: 'ready', ready: true }
    : { status: 'not_ready', ready: false }

  return NextResponse.json(body, {
    status: result.ready ? 200 : 503,
    headers: { 'Cache-Control': 'no-store' },
  })
}
