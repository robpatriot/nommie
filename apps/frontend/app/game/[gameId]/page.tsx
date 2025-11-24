import { auth } from '@/auth'
import { redirect } from 'next/navigation'

import { GameRoomClient } from './_components/game-room-client'
import ErrorBoundary from '@/components/ErrorBoundary'
import { BreadcrumbSetter } from '@/components/header-breadcrumbs'
import { fetchGameSnapshot } from '@/lib/api/game-room'
import { DEFAULT_VIEWER_SEAT } from '@/lib/game-room/constants'
import { extractPlayerNames } from '@/utils/player-names'
import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import { getUserOptions } from '@/lib/api/user-options'

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

  const snapshotResult = await fetchGameSnapshot(resolvedGameId)

  if (snapshotResult.kind !== 'ok') {
    // 'not_modified' should not occur on initial page load without an ETag
    throw new Error('Failed to load game snapshot: unexpected response')
  }

  const seating = snapshotResult.snapshot.game.seating
  const playerNames = extractPlayerNames(seating)

  const hostSeat = (snapshotResult.snapshot.game.host_seat ??
    DEFAULT_VIEWER_SEAT) as typeof DEFAULT_VIEWER_SEAT
  const viewerSeat =
    typeof snapshotResult.viewerSeat === 'number'
      ? (snapshotResult.viewerSeat as typeof DEFAULT_VIEWER_SEAT)
      : DEFAULT_VIEWER_SEAT

  const initialPayload: GameRoomSnapshotPayload = {
    snapshot: snapshotResult.snapshot,
    etag: snapshotResult.etag,
    playerNames,
    viewerSeat,
    viewerHand: snapshotResult.viewerHand ?? [],
    timestamp: new Date().toISOString(),
    hostSeat,
    bidConstraints: snapshotResult.bidConstraints ?? null,
  }

  let requireCardConfirmation = true
  try {
    const options = await getUserOptions()
    requireCardConfirmation = options.require_card_confirmation
  } catch {
    // Fallback to default behavior if options cannot be loaded
  }

  const gameName = `Game ${resolvedGameId}`

  return (
    <ErrorBoundary>
      <BreadcrumbSetter
        crumbs={[{ label: 'Lobby', href: '/lobby' }, { label: gameName }]}
      />
      <GameRoomClient
        initialData={initialPayload}
        gameId={resolvedGameId}
        requireCardConfirmation={requireCardConfirmation}
      />
    </ErrorBoundary>
  )
}
