import { auth } from '@/auth'
import { redirect } from 'next/navigation'

import { GameRoomClient } from './_components/game-room-client'
import { getMockGameRoomData } from '@/lib/game-room/mock-data'
import { fetchGameSnapshot } from '@/lib/api/game-room'
import { DEFAULT_VIEWER_SEAT } from '@/lib/game-room/constants'
import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'

interface GamePageProps {
  params: Promise<{
    gameId: string
  }>
}

export default async function GamePage({ params }: GamePageProps) {
  const session = await auth()

  // Protect route: redirect to home if not authenticated
  if (!session) {
    redirect('/')
  }

  const { gameId } = await params
  const numericGameId = Number(gameId)
  const resolvedGameId = Number.isNaN(numericGameId) ? 0 : numericGameId

  let initialPayload: GameRoomSnapshotPayload | null = null
  let initialError: { message: string; traceId?: string } | null = null

  try {
    const snapshotResult = await fetchGameSnapshot(resolvedGameId)

    if (snapshotResult.kind === 'ok') {
      const seating = snapshotResult.snapshot.game.seating
      const playerNames = seating.map((seat, index) => {
        const name = seat.display_name?.trim()
        if (name && name.length > 0) {
          return name
        }
        return `Seat ${index + 1}`
      }) as [string, string, string, string]

      const hostSeat = (snapshotResult.snapshot.game.host_seat ??
        DEFAULT_VIEWER_SEAT) as typeof DEFAULT_VIEWER_SEAT
      const viewerSeat =
        typeof snapshotResult.viewerSeat === 'number'
          ? (snapshotResult.viewerSeat as typeof DEFAULT_VIEWER_SEAT)
          : DEFAULT_VIEWER_SEAT

      initialPayload = {
        snapshot: snapshotResult.snapshot,
        etag: snapshotResult.etag,
        playerNames,
        viewerSeat,
        viewerHand: [],
        timestamp: new Date().toISOString(),
        hostSeat,
      }
    }
  } catch (error) {
    console.warn(
      'Failed to load game snapshot, falling back to mock data',
      error
    )
    initialError = {
      message:
        error instanceof Error
          ? error.message
          : 'Unable to load live game snapshot. Showing demo view instead.',
    }
  }

  if (!initialPayload) {
    const mock = getMockGameRoomData(resolvedGameId)
    initialPayload = {
      snapshot: mock.snapshot,
      etag: undefined,
      playerNames: mock.playerNames,
      viewerSeat: mock.viewerSeat,
      viewerHand: mock.viewerHand,
      timestamp: mock.lastSyncedAt,
      hostSeat: mock.hostSeat,
    }
  }

  return (
    <GameRoomClient
      initialData={initialPayload}
      initialError={initialError}
      gameId={resolvedGameId}
    />
  )
}
