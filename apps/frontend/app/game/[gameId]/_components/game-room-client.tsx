'use client'

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import {
  getGameRoomSnapshotAction,
  markPlayerReadyAction,
  submitBidAction,
  selectTrumpAction,
  submitPlayAction,
  addAiSeatAction,
  updateAiSeatAction,
  removeAiSeatAction,
  fetchAiRegistryAction,
} from '@/app/actions/game-room-actions'
import Toast, { type ToastMessage } from '@/components/Toast'
import { BackendApiError } from '@/lib/errors'
import type { Seat, Trump } from '@/lib/game-room/types'

import { GameRoomView, type AiSeatSelection } from './game-room-view'

const DEFAULT_AI_NAME = 'HeuristicV1'

type AiRegistryEntryState = {
  name: string
  version: string
}

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
  const [isTrumpPending, setIsTrumpPending] = useState(false)
  const [isPlayPending, setIsPlayPending] = useState(false)
  const [isAiPending, setIsAiPending] = useState(false)
  const [hasMarkedReady, setHasMarkedReady] = useState(false)
  const [toast, setToast] = useState<ToastMessage | null>(null)
  const [aiRegistry, setAiRegistry] = useState<AiRegistryEntryState[]>([])
  const [isAiRegistryLoading, setIsAiRegistryLoading] = useState(false)
  const [aiRegistryError, setAiRegistryError] = useState<string | null>(null)
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
          setSnapshot((prev) => ({
            ...result.data,
            viewerSeat:
              result.data.viewerSeat !== null
                ? result.data.viewerSeat
                : prev.viewerSeat,
          }))
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
        const handleFailure = (failure: unknown) => {
          if (failure instanceof Error) {
            setError({ message: failure.message })
          } else {
            setError({ message: 'Unable to refresh game state' })
          }
        }

        const shouldRetry =
          err instanceof Error &&
          (err.message === 'fetch failed' || err.message === 'network timeout')

        if (shouldRetry) {
          try {
            const retryResult = await getGameRoomSnapshotAction({ gameId })

            if (retryResult.kind === 'ok') {
              setSnapshot((prev) => ({
                ...retryResult.data,
                viewerSeat:
                  retryResult.data.viewerSeat !== null
                    ? retryResult.data.viewerSeat
                    : prev.viewerSeat,
              }))
              setEtag(retryResult.data.etag)
              setError(null)
              inflightRef.current = false
              setIsPolling(false)
              setIsRefreshing(false)
              return
            }

            if (retryResult.kind === 'not_modified') {
              setSnapshot((prev) => ({
                ...prev,
                timestamp: new Date().toISOString(),
              }))
              setError(null)
              inflightRef.current = false
              setIsPolling(false)
              setIsRefreshing(false)
              return
            }

            setError({
              message: retryResult.message,
              traceId: retryResult.traceId,
            })
          } catch (retryErr) {
            console.error('Snapshot refresh retry failed', retryErr)
            handleFailure(retryErr)
          }
        } else {
          console.error('Snapshot refresh failed', err)
          handleFailure(err)
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

  const handleSelectTrump = useCallback(
    async (trump: Trump) => {
      if (isTrumpPending) {
        return
      }

      setIsTrumpPending(true)

      try {
        const result = await selectTrumpAction({
          gameId,
          trump,
        })

        if (result.kind === 'error') {
          const actionError = new BackendApiError(
            result.message || 'Failed to select trump',
            result.status,
            undefined,
            result.traceId
          )

          showToast(actionError.message, 'error', actionError)

          if (process.env.NODE_ENV === 'development' && actionError.traceId) {
            console.error('Select trump error traceId:', actionError.traceId)
          }

          return
        }

        showToast('Trump selected', 'success')
        await performRefresh('manual')
      } catch (err) {
        const message =
          err instanceof Error ? err.message : 'Unable to select trump'
        const wrappedError =
          err instanceof BackendApiError
            ? err
            : new BackendApiError(message, 500, 'UNKNOWN_ERROR')

        showToast(wrappedError.message, 'error', wrappedError)

        if (process.env.NODE_ENV === 'development' && wrappedError.traceId) {
          console.error('Select trump error traceId:', wrappedError.traceId)
        }
      } finally {
        setIsTrumpPending(false)
      }
    },
    [gameId, isTrumpPending, performRefresh, showToast]
  )

  const handlePlayCard = useCallback(
    async (card: string) => {
      if (isPlayPending) {
        return
      }

      setIsPlayPending(true)

      try {
        const result = await submitPlayAction({
          gameId,
          card,
        })

        if (result.kind === 'error') {
          const actionError = new BackendApiError(
            result.message || 'Failed to play card',
            result.status,
            undefined,
            result.traceId
          )

          showToast(actionError.message, 'error', actionError)

          if (process.env.NODE_ENV === 'development' && actionError.traceId) {
            console.error('Play card error traceId:', actionError.traceId)
          }

          return
        }

        showToast('Card played', 'success')
        await performRefresh('manual')
      } catch (err) {
        const message =
          err instanceof Error ? err.message : 'Unable to play card'
        const wrappedError =
          err instanceof BackendApiError
            ? err
            : new BackendApiError(message, 500, 'UNKNOWN_ERROR')

        showToast(wrappedError.message, 'error', wrappedError)

        if (process.env.NODE_ENV === 'development' && wrappedError.traceId) {
          console.error('Play card error traceId:', wrappedError.traceId)
        }
      } finally {
        setIsPlayPending(false)
      }
    },
    [gameId, isPlayPending, performRefresh, showToast]
  )

  const viewerSeatForInteractions =
    typeof snapshot.viewerSeat === 'number' ? snapshot.viewerSeat : null

  const phase = snapshot.snapshot.phase

  const biddingControls = useMemo(() => {
    if (
      phase.phase !== 'Bidding' ||
      viewerSeatForInteractions === null ||
      phase.data.bids[viewerSeatForInteractions] !== null
    ) {
      return undefined
    }

    return {
      viewerSeat: viewerSeatForInteractions,
      isPending: isBidPending,
      onSubmit: handleSubmitBid,
    }
  }, [handleSubmitBid, isBidPending, phase, viewerSeatForInteractions])

  const trumpControls = useMemo(() => {
    if (phase.phase !== 'TrumpSelect') {
      return undefined
    }

    if (viewerSeatForInteractions === null) {
      return undefined
    }

    const allowedTrumps = phase.data.allowed_trumps
    const toAct = phase.data.to_act
    const canSelect = toAct === viewerSeatForInteractions

    return {
      viewerSeat: viewerSeatForInteractions,
      toAct,
      allowedTrumps,
      canSelect,
      isPending: isTrumpPending,
      onSelect: canSelect
        ? (trump: Trump) => {
            void handleSelectTrump(trump)
          }
        : undefined,
    }
  }, [handleSelectTrump, isTrumpPending, phase, viewerSeatForInteractions])

  const playControls = useMemo(() => {
    if (phase.phase !== 'Trick') {
      return undefined
    }

    if (viewerSeatForInteractions === null) {
      return undefined
    }

    const playable = phase.data.playable

    return {
      viewerSeat: viewerSeatForInteractions,
      playable,
      isPending: isPlayPending,
      onPlay: handlePlayCard,
    }
  }, [handlePlayCard, isPlayPending, phase, viewerSeatForInteractions])

  const seatInfo = useMemo(() => {
    return snapshot.snapshot.game.seating.map((seat, index) => {
      const seatIndex =
        typeof seat.seat === 'number' && !Number.isNaN(seat.seat)
          ? (seat.seat as Seat)
          : (index as Seat)

      const normalizedName = seat.display_name?.trim()
      const name =
        normalizedName && normalizedName.length > 0
          ? normalizedName
          : `Seat ${seatIndex + 1}`

      return {
        seat: seatIndex,
        name,
        userId: seat.user_id,
        isOccupied: Boolean(seat.user_id),
        isAi: seat.is_ai,
        isReady: seat.is_ready,
        aiProfile: seat.ai_profile ?? null,
      }
    })
  }, [snapshot.snapshot.game.seating])

  const totalSeats = seatInfo.length
  const occupiedSeats = seatInfo.filter((seat) => seat.isOccupied).length
  const aiSeats = seatInfo.filter((seat) => seat.isAi).length
  const availableSeats = totalSeats - occupiedSeats

  const hostSeat: Seat = snapshot.hostSeat
  const viewerIsHost = viewerSeatForInteractions === hostSeat
  const canManageAi =
    viewerIsHost && phase.phase === 'Init' && !isRefreshing && !isPolling

  useEffect(() => {
    if (!canManageAi) {
      setAiRegistry([])
      setAiRegistryError(null)
      setIsAiRegistryLoading(false)
      return
    }

    let cancelled = false
    setIsAiRegistryLoading(true)
    setAiRegistryError(null)

    void fetchAiRegistryAction()
      .then((result) => {
        if (cancelled) {
          return
        }

        if (result.kind === 'ok') {
          setAiRegistry(result.ais)
        } else {
          setAiRegistryError(result.message)
        }
      })
      .catch((err) => {
        if (cancelled) {
          return
        }

        const message =
          err instanceof Error ? err.message : 'Failed to load AI registry'
        setAiRegistryError(message)
      })
      .finally(() => {
        if (!cancelled) {
          setIsAiRegistryLoading(false)
        }
      })

    return () => {
      cancelled = true
    }
  }, [canManageAi])

  const handleAddAi = useCallback(
    async (selection?: AiSeatSelection) => {
      if (isAiPending || !canManageAi) {
        return
      }

      setIsAiPending(true)

      try {
        const registryName =
          selection?.registryName ??
          aiRegistry.find((entry) => entry.name === DEFAULT_AI_NAME)?.name ??
          DEFAULT_AI_NAME
        const registryVersion =
          selection?.registryVersion ??
          aiRegistry.find((entry) => entry.name === registryName)?.version

        const result = await addAiSeatAction({
          gameId,
          registryName,
          registryVersion,
          seed: selection?.seed,
        })

        if (result.kind === 'error') {
          const actionError = new BackendApiError(
            result.message || 'Failed to add AI seat',
            result.status,
            undefined,
            result.traceId
          )

          showToast(actionError.message, 'error', actionError)

          if (process.env.NODE_ENV === 'development' && actionError.traceId) {
            console.error('Add AI seat error traceId:', actionError.traceId)
          }

          return
        }

        showToast('AI seat added', 'success')
        await performRefresh('manual')
      } catch (err) {
        const message =
          err instanceof Error ? err.message : 'Unable to add AI seat'
        const wrappedError =
          err instanceof BackendApiError
            ? err
            : new BackendApiError(message, 500, 'UNKNOWN_ERROR')

        showToast(wrappedError.message, 'error', wrappedError)

        if (process.env.NODE_ENV === 'development' && wrappedError.traceId) {
          console.error('Add AI seat error traceId:', wrappedError.traceId)
        }
      } finally {
        setIsAiPending(false)
      }
    },
    [aiRegistry, canManageAi, gameId, isAiPending, performRefresh, showToast]
  )

  const handleRemoveAiSeat = useCallback(
    async (seat: Seat) => {
      if (isAiPending || !canManageAi) {
        return
      }

      setIsAiPending(true)

      try {
        const result = await removeAiSeatAction({
          gameId,
          seat,
        })

        if (result.kind === 'error') {
          const actionError = new BackendApiError(
            result.message || 'Failed to remove AI seat',
            result.status,
            undefined,
            result.traceId
          )

          showToast(actionError.message, 'error', actionError)

          if (process.env.NODE_ENV === 'development' && actionError.traceId) {
            console.error('Remove AI seat error traceId:', actionError.traceId)
          }

          return
        }

        showToast('AI seat removed', 'success')
        await performRefresh('manual')
      } catch (err) {
        const message =
          err instanceof Error ? err.message : 'Unable to remove AI seat'
        const wrappedError =
          err instanceof BackendApiError
            ? err
            : new BackendApiError(message, 500, 'UNKNOWN_ERROR')

        showToast(wrappedError.message, 'error', wrappedError)

        if (process.env.NODE_ENV === 'development' && wrappedError.traceId) {
          console.error('Remove AI seat error traceId:', wrappedError.traceId)
        }
      } finally {
        setIsAiPending(false)
      }
    },
    [canManageAi, gameId, isAiPending, performRefresh, showToast]
  )

  const handleUpdateAiSeat = useCallback(
    async (seat: Seat, selection: AiSeatSelection) => {
      if (isAiPending || !canManageAi) {
        return
      }

      setIsAiPending(true)

      try {
        const result = await updateAiSeatAction({
          gameId,
          seat,
          registryName: selection.registryName,
          registryVersion: selection.registryVersion,
          seed: selection.seed,
        })

        if (result.kind === 'error') {
          const actionError = new BackendApiError(
            result.message || 'Failed to update AI seat',
            result.status,
            undefined,
            result.traceId
          )

          showToast(actionError.message, 'error', actionError)

          if (process.env.NODE_ENV === 'development' && actionError.traceId) {
            console.error('Update AI seat error traceId:', actionError.traceId)
          }

          return
        }

        showToast('AI seat updated', 'success')
        await performRefresh('manual')
      } catch (err) {
        const message =
          err instanceof Error ? err.message : 'Unable to update AI seat'
        const wrappedError =
          err instanceof BackendApiError
            ? err
            : new BackendApiError(message, 500, 'UNKNOWN_ERROR')

        showToast(wrappedError.message, 'error', wrappedError)

        if (process.env.NODE_ENV === 'development' && wrappedError.traceId) {
          console.error('Update AI seat error traceId:', wrappedError.traceId)
        }
      } finally {
        setIsAiPending(false)
      }
    },
    [canManageAi, gameId, isAiPending, performRefresh, showToast]
  )

  const aiSeatState = useMemo(() => {
    if (!canManageAi) {
      return undefined
    }

    return {
      totalSeats,
      availableSeats,
      aiSeats,
      isPending: isAiPending,
      canAdd: availableSeats > 0 && !isAiRegistryLoading,
      canRemove: aiSeats > 0,
      onAdd: (selection?: AiSeatSelection) => {
        void handleAddAi(selection)
      },
      onRemoveSeat: (seat: Seat) => {
        void handleRemoveAiSeat(seat)
      },
      onUpdateSeat: (seat: Seat, selection: AiSeatSelection) => {
        void handleUpdateAiSeat(seat, selection)
      },
      registry: {
        entries: aiRegistry,
        isLoading: isAiRegistryLoading,
        error: aiRegistryError,
        defaultName: DEFAULT_AI_NAME,
      },
      seats: seatInfo,
    }
  }, [
    aiRegistry,
    aiRegistryError,
    aiSeats,
    availableSeats,
    canManageAi,
    handleAddAi,
    handleRemoveAiSeat,
    handleUpdateAiSeat,
    isAiPending,
    isAiRegistryLoading,
    seatInfo,
    totalSeats,
  ])

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
        trumpState={trumpControls}
        playState={playControls}
        aiSeatState={aiSeatState}
      />
      <Toast toast={toast} onClose={() => setToast(null)} />
    </>
  )
}
