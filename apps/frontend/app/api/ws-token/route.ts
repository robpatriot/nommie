import { NextResponse } from 'next/server'

import { getBackendBaseUrlOrThrow } from '@/auth'
import { requireBackendJwt } from '@/lib/server/get-backend-jwt'

export async function GET() {
  try {
    const jwt = await requireBackendJwt()
    const backendBase = getBackendBaseUrlOrThrow()

    const response = await fetch(`${backendBase}/api/ws/token`, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${jwt}`,
        'Content-Type': 'application/json',
      },
      cache: 'no-store',
    })

    if (!response.ok) {
      console.error('Failed to fetch websocket token', response.status)
      return NextResponse.json(
        { error: 'Unable to issue websocket token' },
        { status: response.status }
      )
    }

    const payload = await response.json()
    return NextResponse.json(payload, { status: 200 })
  } catch (error) {
    console.error('Unexpected websocket token error', error)
    return NextResponse.json(
      { error: 'Unable to issue websocket token' },
      { status: 500 }
    )
  }
}
