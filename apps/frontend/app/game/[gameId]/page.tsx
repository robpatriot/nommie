import { getTranslations } from 'next-intl/server'
import { auth } from '@/auth'
import { redirect } from 'next/navigation'

import { GameRoomClient } from './_components/game-room-client'
import { GameJoinErrorClient } from './_components/GameJoinErrorClient'
import ErrorBoundaryWithTranslations from '@/components/ErrorBoundaryWithTranslations'
import { BreadcrumbSetter } from '@/components/header-breadcrumbs'
import { fetchGameState } from '@/lib/api/game-room'
import { getUserOptions } from '@/lib/api/user-options'
import { BackendApiError } from '@/lib/api'
import { isInStartupWindow } from '@/lib/server/backend-status'
import { isBackendStartupError } from '@/lib/server/connection-errors'
import { handleStaleSessionError } from '@/lib/auth/allowlist'
import {
  gameStateMsgToRoomState,
  selectViewerSeat,
} from '@/lib/game-room/state'

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
  let fetchResult
  try {
    fetchResult = await fetchGameState(resolvedGameId)
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

  if (fetchResult.kind !== 'ok') {
    // 'not_modified' should not occur on initial page load without an ETag
    const t = await getTranslations('errors.page')
    throw new Error(t('failedToLoadGameState'))
  }

  const initialState = {
    ...gameStateMsgToRoomState(fetchResult.msg, {
      source: 'http',
      receivedAt: new Date().toISOString(),
    }),
    ...(fetchResult.etag && { etag: fetchResult.etag }),
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
  const isSpectator = selectViewerSeat(initialState) === null
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
        initialState={initialState}
        gameId={resolvedGameId}
        requireCardConfirmation={requireCardConfirmation}
        trickDisplayDurationSeconds={trickDisplayDurationSeconds}
      />
    </ErrorBoundaryWithTranslations>
  )
}
