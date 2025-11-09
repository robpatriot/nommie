'use client'

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import {
  getGameRoomSnapshotAction,
  markPlayerReadyAction,
  submitBidAction,
} from '@/app/actions/game-room-actions'
import Toast, { type ToastMessage } from '@/components/Toast'
import { BackendApiError } from '@/lib/errors'

import { GameRoomView } from './game-room-view'

interface GameRoomClientProps {
  initialData: GameRoomSnapshotPayload
  gameId: number
  pollingMs?: number
  initialError?: { message: string; traceId?: string } | null
}

export function GameRoomClient({
  initialData,
  gameId,
  pollingMs = 3000,
  initialError = null,
}: GameRoomClientProps) {
  const [snapshot, setSnapshot] = useState(initialData)
  const [etag, setEtag] = useState<string | undefined>(initialData.etag)
  const [error, setError] = useState<{
    message: string
    traceId?: string
  } | null>(initialError)
  const [isRefreshing, setIsRefreshing] = useState(false)
  const [isPolling, setIsPolling] = useState(false)
  const [isReadyPending, setIsReadyPending] = useState(false)
  const [isBidPending, setIsBidPending] = useState(false)
  const [hasMarkedReady, setHasMarkedReady] = useState(false)
  const [toast, setToast] = useState<ToastMessage | null>(null)
  const inflightRef = useRef(false)

  const performRefresh = useCallback(
    async (mode: 'manual' | 'poll') => {
      if (inflightRef.current) {
        return
      }

      inflightRef.current = true

      if (mode === 'poll') {
        setIsPolling(true)
      } else {
        setIsRefreshing(true)
      }

      try {
        const result = await getGameRoomSnapshotAction({
          gameId,
          etag,
        })

        if (result.kind === 'ok') {
          setSnapshot(result.data)
          setEtag(result.data.etag)
          setError(null)
        } else if (result.kind === 'not_modified') {
          setSnapshot((prev) => ({
            ...prev,
            timestamp: new Date().toISOString(),
          }))
          setError(null)
        } else {
          setError({ message: result.message, traceId: result.traceId })
        }
      } catch (err) {
        if (err instanceof Error) {
          setError({ message: err.message })
        } else {
          setError({ message: 'Unable to refresh game state' })
        }
      } finally {
        setIsPolling(false)
        setIsRefreshing(false)
        inflightRef.current = false
      }
    },
    [etag, gameId]
  )

  useEffect(() => {
    const timer = setInterval(() => {
      void performRefresh('poll')
    }, pollingMs)

    return () => clearInterval(timer)
  }, [performRefresh, pollingMs])

  const status = useMemo(
    () => ({
      lastSyncedAt: snapshot.timestamp,
      isPolling: isPolling || isRefreshing,
    }),
    [snapshot.timestamp, isPolling, isRefreshing]
  )

  const phaseName = snapshot.snapshot.phase.phase
  const canMarkReady = phaseName === 'Init'

  useEffect(() => {
    if (!canMarkReady && hasMarkedReady) {
      setHasMarkedReady(false)
    }
  }, [canMarkReady, hasMarkedReady])

  const showToast = useCallback(
    (message: string, type: ToastMessage['type'], error?: BackendApiError) => {
      setToast({
        id: Date.now().toString(),
        message,
        type,
        error,
      })
    },
    []
  )

  const markReady = useCallback(async () => {
    if (!canMarkReady || isReadyPending || hasMarkedReady) {
      return
    }

    setIsReadyPending(true)

    try {
      const result = await markPlayerReadyAction(gameId)
      if (result.kind === 'error') {
        setError({ message: result.message, traceId: result.traceId })
        return
      }

      setHasMarkedReady(true)
      await performRefresh('manual')
    } catch (err) {
      if (err instanceof Error) {
        setError({ message: err.message })
      } else {
        setError({ message: 'Unable to mark ready' })
      }
    } finally {
      setIsReadyPending(false)
    }
  }, [canMarkReady, gameId, hasMarkedReady, isReadyPending, performRefresh])

  const handleSubmitBid = useCallback(
    async (bid: number) => {
      if (isBidPending) {
        return
      }

      setIsBidPending(true)

      try {
        const result = await submitBidAction({
          gameId,
          bid,
        })

        if (result.kind === 'error') {
          const actionError = new BackendApiError(
            result.message || 'Failed to submit bid',
            result.status,
            undefined,
            result.traceId
          )

          showToast(actionError.message, 'error', actionError)

          if (process.env.NODE_ENV === 'development' && actionError.traceId) {
            console.error('Submit bid error traceId:', actionError.traceId)
          }

          return
        }

        showToast('Bid submitted', 'success')
        await performRefresh('manual')
      } catch (err) {
        const message =
          err instanceof Error ? err.message : 'Unable to submit bid'
        const wrappedError =
          err instanceof BackendApiError
            ? err
            : new BackendApiError(message, 500, 'UNKNOWN_ERROR')

        showToast(wrappedError.message, 'error', wrappedError)

        if (process.env.NODE_ENV === 'development' && wrappedError.traceId) {
          console.error('Submit bid error traceId:', wrappedError.traceId)
        }
      } finally {
        setIsBidPending(false)
      }
    },
    [gameId, isBidPending, performRefresh, showToast]
  )

  const viewerSeatForInteractions =
    typeof snapshot.viewerSeat === 'number' ? snapshot.viewerSeat : null

  const phase = snapshot.snapshot.phase

  const biddingControls = useMemo(() => {
    if (phase.phase !== 'Bidding') {
      return undefined
    }

    if (viewerSeatForInteractions === null) {
      return undefined
    }

    return {
      viewerSeat: viewerSeatForInteractions,
      isPending: isBidPending,
      onSubmit: handleSubmitBid,
    }
  }, [handleSubmitBid, isBidPending, phase, viewerSeatForInteractions])

  return (
    <>
      <GameRoomView
        gameId={gameId}
        snapshot={snapshot.snapshot}
        playerNames={snapshot.playerNames}
        viewerSeat={snapshot.viewerSeat ?? undefined}
        viewerHand={snapshot.viewerHand}
        status={status}
        onRefresh={() => void performRefresh('manual')}
        isRefreshing={isRefreshing}
        error={error}
        readyState={{
          canReady: canMarkReady,
          isPending: isReadyPending,
          hasMarked: hasMarkedReady,
          onReady: () => {
            void markReady()
          },
        }}
        biddingState={biddingControls}
      />
      <Toast toast={toast} onClose={() => setToast(null)} />
    </>
  )
}
