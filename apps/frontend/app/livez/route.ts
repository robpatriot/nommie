import { NextResponse } from 'next/server'

export async function GET() {
  return NextResponse.json(
    { status: 'alive' },
    {
      status: 200,
      headers: { 'Cache-Control': 'no-store' },
    }
  )
}
