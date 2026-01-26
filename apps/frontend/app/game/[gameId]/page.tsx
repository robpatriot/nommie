import { getTranslations } from 'next-intl/server'
import { auth } from '@/auth'
import { redirect } from 'next/navigation'

import { GameRoomClient } from './_components/game-room-client'
import { GameJoinErrorClient } from './_components/GameJoinErrorClient'
import ErrorBoundaryWithTranslations from '@/components/ErrorBoundaryWithTranslations'
import { BreadcrumbSetter } from '@/components/header-breadcrumbs'
import { fetchGameSnapshot } from '@/lib/api/game-room'
import { extractPlayerNames } from '@/utils/player-names'
import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import { getUserOptions } from '@/lib/api/user-options'
import { BackendApiError } from '@/lib/api'
import { isInStartupWindow } from '@/lib/server/backend-status'
import { isBackendStartupError } from '@/lib/server/connection-errors'
import { handleStaleSessionError } from '@/lib/auth/allowlist'
import { gameStateMsgToSnapshotPayload } from '@/lib/game-room/protocol/transform'

export default async function GamePage({
  params,
}: {
  params: Promise<{ gameId: string }>
}) {
  const session = await auth()

  // Protect route: redirect to home if not authenticated
  if (!session) {
    redirect('/')
  }

  const { gameId } = await params
  const numericGameId = Number(gameId)
  const resolvedGameId = Number.isNaN(numericGameId) ? 0 : numericGameId

  // Fetch game snapshot, but handle backend startup gracefully
  // Error handling is centralized in fetchWithAuth - errors during startup
  // are suppressed automatically. If backend is starting up, redirect to lobby.
  let snapshotResult
  try {
    snapshotResult = await fetchGameSnapshot(resolvedGameId)
  } catch (error) {
    await handleStaleSessionError(error)
    if (error instanceof BackendApiError) {
      const isStartupError =
        error.status === 503 ||
        error.code === 'BACKEND_STARTING' ||
        isBackendStartupError(error, isInStartupWindow)

      if (isStartupError) {
        redirect('/lobby')
      }

      return <GameJoinErrorClient code={error.code} status={error.status} />
    }

    if (isBackendStartupError(error, isInStartupWindow)) {
      redirect('/lobby')
    }

    throw error
  }

  if (snapshotResult.kind !== 'ok') {
    // 'not_modified' should not occur on initial page load without an ETag
    const t = await getTranslations('errors.page')
    throw new Error(t('failedToLoadGameSnapshot'))
  }

  const payload = gameStateMsgToSnapshotPayload(snapshotResult.msg, {
    etag: snapshotResult.etag,
    timestamp: new Date().toISOString(),
  })

  const seating = payload.snapshot.game.seating
  const playerNames = extractPlayerNames(seating)

  const { hostSeat, viewerSeat } = payload

  const initialPayload: GameRoomSnapshotPayload = {
    snapshot: payload.snapshot,
    etag: snapshotResult.etag,
    version: payload.version,
    playerNames,
    viewerSeat,
    viewerHand: payload.viewerHand,
    timestamp: new Date().toISOString(),
    hostSeat,
    bidConstraints: payload.bidConstraints,
  }

  let requireCardConfirmation = true
  let trickDisplayDurationSeconds: number | null = null
  try {
    const options = await getUserOptions()
    requireCardConfirmation = options.require_card_confirmation
    trickDisplayDurationSeconds = options.trick_display_duration_seconds
  } catch (error) {
    await handleStaleSessionError(error)
    // Fallback to default behavior if options cannot be loaded
  }

  const t = await getTranslations('common')
  const tLobby = await getTranslations('lobby')
  const isSpectator = payload.viewerSeat === null
  const gameName = isSpectator
    ? `${t('gameName', { gameId: resolvedGameId })} (spectating)`
    : t('gameName', { gameId: resolvedGameId })

  return (
    <ErrorBoundaryWithTranslations>
      <BreadcrumbSetter
        crumbs={[
          { label: tLobby('breadcrumbs.lobby'), href: '/lobby' },
          { label: gameName },
        ]}
      />
      <GameRoomClient
        initialData={initialPayload}
        gameId={resolvedGameId}
        requireCardConfirmation={requireCardConfirmation}
        trickDisplayDurationSeconds={trickDisplayDurationSeconds}
      />
    </ErrorBoundaryWithTranslations>
  )
}
