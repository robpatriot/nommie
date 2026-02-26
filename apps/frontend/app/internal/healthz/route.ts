import { NextResponse } from 'next/server'

export async function GET() {
  const uptimeMs = process.uptime() * 1000

  return NextResponse.json(
    {
      service: 'frontend',
      status: 'alive',
      uptime_seconds: Math.floor(uptimeMs / 1000),
    },
    {
      status: 200,
      headers: { 'Cache-Control': 'no-store' },
    }
  )
}
