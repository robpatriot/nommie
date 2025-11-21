'use server'

import { NextResponse } from 'next/server'

import { fetchWithAuth, BackendApiError } from '@/lib/api'

interface RouteParams {
  params: {
    gameId: string
  }
}

export async function GET(_request: Request, { params }: RouteParams) {
  const parsedGameId = Number.parseInt(params.gameId, 10)

  if (!Number.isFinite(parsedGameId) || parsedGameId <= 0) {
    return NextResponse.json(
      { message: 'Invalid game id' },
      {
        status: 400,
      }
    )
  }

  try {
    const response = await fetchWithAuth(`/api/games/${parsedGameId}/history`)
    const payload = await response.json()

    return NextResponse.json(payload)
  } catch (error) {
    if (error instanceof BackendApiError) {
      return NextResponse.json(
        {
          message: error.message,
          code: error.code,
          traceId: error.traceId,
        },
        { status: error.status }
      )
    }

    console.error('Failed to proxy game history', error)
    return NextResponse.json(
      { message: 'Failed to fetch score history' },
      { status: 500 }
    )
  }
}
