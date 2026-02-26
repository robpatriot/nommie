import { NextResponse } from 'next/server'

import { checkBackendReadiness } from '@/lib/server/backend-health'

export async function GET() {
  const result = await checkBackendReadiness()

  const body = result.ready
    ? { status: 'ready', ready: true }
    : { status: 'not_ready', ready: false }

  return NextResponse.json(body, {
    status: result.ready ? 200 : 503,
    headers: { 'Cache-Control': 'no-store' },
  })
}
