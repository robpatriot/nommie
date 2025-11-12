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
import Toast from '@/components/Toast'
import { useApiAction } from '@/hooks/useApiAction'
import { useToast } from '@/hooks/useToast'
import type { Seat, Trump } from '@/lib/game-room/types'

import { GameRoomView, type AiSeatSelection } from './game-room-view'

const DEFAULT_AI_NAME = 'HeuristicV1'

type AiRegistryEntryState = {
  name: string
  version: string
}

/**
 * Unified activity state tracking for polling, refresh, and actions.
 * Only one activity can be in progress at a time.
 */
type ActivityState =
  | { type: 'idle' }
  | { type: 'polling' }
  | { type: 'refreshing' }
  | { type: 'action'; action: 'ready' | 'bid' | 'trump' | 'play' | 'ai' }

interface GameRoomClientProps {
  initialData: GameRoomSnapshotPayload
  gameId: number
  pollingMs?: number
}

export function GameRoomClient({
  initialData,
  gameId,
  pollingMs = 3000,
}: GameRoomClientProps) {
  const [snapshot, setSnapshot] = useState(initialData)
  const [etag, setEtag] = useState<string | undefined>(initialData.etag)
  const [error, setError] = useState<{
    message: string
    traceId?: string
  } | null>(null)
  const [activity, setActivity] = useState<ActivityState>({ type: 'idle' })
  const [hasMarkedReady, setHasMarkedReady] = useState(false)
  const { toast, showToast, hideToast } = useToast()
  const [aiRegistry, setAiRegistry] = useState<AiRegistryEntryState[]>([])
  const [isAiRegistryLoading, setIsAiRegistryLoading] = useState(false)
  const [aiRegistryError, setAiRegistryError] = useState<string | null>(null)
  // Refs for concurrent operations:
  // - inflightRef: Tracks if an API call is in progress (used to prevent concurrent requests)
  // - activityRef: Current activity state as ref (used to avoid stale closures in callbacks)
  // - pendingManualRefreshRef: Flags if a manual refresh should run after current operation completes
  // Why both state and refs? React state updates are asynchronous, but we need synchronous
  // checks in callbacks (like performRefresh) to avoid race conditions. The ref provides
  // the current value immediately, while state triggers re-renders for UI updates.
  const inflightRef = useRef(false)
  const activityRef = useRef<ActivityState>(activity)
  const pendingManualRefreshRef = useRef(false)

  // Keep ref in sync with state so callbacks always see current activity
  useEffect(() => {
    activityRef.current = activity
  }, [activity])

  // Derived state for convenience
  const isIdle = activity.type === 'idle'
  const isRefreshing = activity.type === 'refreshing'
  const isReadyPending =
    activity.type === 'action' && activity.action === 'ready'
  const isBidPending = activity.type === 'action' && activity.action === 'bid'
  const isTrumpPending =
    activity.type === 'action' && activity.action === 'trump'
  const isPlayPending = activity.type === 'action' && activity.action === 'play'
  const isAiPending = activity.type === 'action' && activity.action === 'ai'
  const isActive = !isIdle

  /**
   * Executes the actual API call to refresh the game snapshot.
   * This is the core refresh logic that handles the HTTP request, retries, and state updates.
   *
   * @param activityType - Whether this is a 'polling' or 'refreshing' operation
   * @returns Promise that resolves when the refresh completes
   */
  const executeRefresh = useCallback(
    async (activityType: 'polling' | 'refreshing' = 'refreshing') => {
      // Prevent concurrent refresh calls
      if (inflightRef.current) {
        return
      }

      inflightRef.current = true
      const newActivity: ActivityState = { type: activityType }
      activityRef.current = newActivity
      setActivity(newActivity)

      try {
        const result = await getGameRoomSnapshotAction({
          gameId,
          etag,
        })

        if (result.kind === 'ok') {
          // Don't preserve viewerSeat from previous snapshot - if API returns null,
          // accept it (fail hard approach). This prevents stale data when viewer's
          // seat changes or API response is missing this field.
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
              // Don't preserve viewerSeat from previous snapshot - fail hard approach
              setSnapshot(retryResult.data)
              setEtag(retryResult.data.etag)
              setError(null)
            } else if (retryResult.kind === 'not_modified') {
              setSnapshot((prev) => ({
                ...prev,
                timestamp: new Date().toISOString(),
              }))
              setError(null)
            } else {
              setError({
                message: retryResult.message,
                traceId: retryResult.traceId,
              })
            }
          } catch (retryErr) {
            console.error('Snapshot refresh retry failed', retryErr)
            handleFailure(retryErr)
          }
        } else {
          console.error('Snapshot refresh failed', err)
          handleFailure(err)
        }
      } finally {
        inflightRef.current = false

        // Check if there's a pending manual refresh that was queued during this operation
        const hasPendingManualRefresh = pendingManualRefreshRef.current
        pendingManualRefreshRef.current = false

        // Handle pending refresh: If a manual refresh was requested while this operation
        // was in progress, trigger it now. This ensures queued refreshes always execute.
        // Note: This is a tail-recursive call, but it's safe because:
        // 1. inflightRef is cleared before the recursive call
        // 2. The recursion depth is bounded (at most one pending refresh)
        // 3. Each call processes one pending refresh and clears the flag
        if (hasPendingManualRefresh) {
          // Keep activity as refreshing and trigger the queued refresh immediately
          activityRef.current = { type: 'refreshing' }
          setActivity({ type: 'refreshing' })
          // Recursive call: execute the pending refresh
          void executeRefresh('refreshing')
        } else {
          // No pending refresh - reset to idle
          activityRef.current = { type: 'idle' }
          setActivity({ type: 'idle' })
        }
      }
    },
    [etag, gameId]
  )

  /**
   * Coordinates refresh requests, handling queuing and coordination between polling and manual refreshes.
   * This is the public API for requesting refreshes - it handles queuing when operations are in progress.
   *
   * @param mode - 'poll' for automatic polling, 'manual' for user-initiated refresh
   */
  const requestRefresh = useCallback(
    async (mode: 'manual' | 'poll') => {
      const currentActivity = activityRef.current

      // Polling: Only poll when idle (no actions or refreshes in progress)
      if (mode === 'poll') {
        if (currentActivity.type !== 'idle' || inflightRef.current) {
          return
        }
        // Start polling refresh
        await executeRefresh('polling')
        return
      }

      // Manual refresh: Queue if there's an operation in progress, execute immediately otherwise
      if (mode === 'manual') {
        // If there's an action in progress, queue the refresh to run after it completes
        if (currentActivity.type === 'action') {
          pendingManualRefreshRef.current = true
          return
        }
        // If there's an inflight API call (refresh/poll), queue manual refresh for after it completes
        if (inflightRef.current) {
          pendingManualRefreshRef.current = true
          // Set activity to refreshing immediately to stop polling after this cycle
          activityRef.current = { type: 'refreshing' }
          setActivity({ type: 'refreshing' })
          return
        }
        // If already refreshing, don't start another
        if (currentActivity.type === 'refreshing') {
          return
        }
      }

      // All checks passed - execute refresh immediately
      await executeRefresh('refreshing')
    },
    [executeRefresh]
  )

  // Polling effect: periodically requests refresh when idle
  useEffect(() => {
    const timer = setInterval(() => {
      // requestRefresh will check activity state internally and only poll when idle
      void requestRefresh('poll')
    }, pollingMs)

    return () => clearInterval(timer)
  }, [requestRefresh, pollingMs])

  // Status derived from activity state
  const status = useMemo(
    () => ({
      lastSyncedAt: snapshot.timestamp,
      isPolling: isActive, // Show active indicator for any activity
    }),
    [snapshot.timestamp, isActive]
  )

  const phase = snapshot.snapshot.phase
  const phaseName = phase.phase
  const canMarkReady = phaseName === 'Init'

  // Reset hasMarkedReady when phase changes away from Init.
  // Use phase directly (not canMarkReady) to avoid race conditions on rapid phase changes.
  useEffect(() => {
    if (phaseName !== 'Init' && hasMarkedReady) {
      setHasMarkedReady(false)
    }
  }, [phaseName, hasMarkedReady])

  const executeApiAction = useApiAction({
    showToast,
    // Don't trigger refresh here - action handlers will do it after activity resets
  })

  /**
   * Shared logic for completing an action and triggering a refresh.
   * This eliminates duplication across all action handlers and ensures consistent behavior.
   *
   * Flow:
   * 1. Reset activity to idle (both state and ref for immediate effect)
   * 2. If a refresh is in progress, queue this refresh for after it completes
   * 3. Otherwise, execute refresh immediately
   */
  const completeActionAndRefresh = useCallback(async () => {
    // Reset activity to idle and update ref immediately (ref needed for synchronous checks)
    activityRef.current = { type: 'idle' }
    setActivity({ type: 'idle' })

    // If there's an inflight refresh (poll), queue this refresh for after it completes
    if (inflightRef.current) {
      pendingManualRefreshRef.current = true
      return
    }

    // Clear any queued refresh flag and execute refresh immediately
    pendingManualRefreshRef.current = false
    await executeRefresh('refreshing')
  }, [executeRefresh])

  const markReady = useCallback(async () => {
    if (!canMarkReady || isReadyPending || hasMarkedReady || !isIdle) {
      return
    }

    setActivity({ type: 'action', action: 'ready' })

    try {
      const result = await markPlayerReadyAction(gameId)
      if (result.kind === 'error') {
        setError({ message: result.message, traceId: result.traceId })
        setActivity({ type: 'idle' })
        activityRef.current = { type: 'idle' }
        return
      }

      setHasMarkedReady(true)
    } catch (err) {
      if (err instanceof Error) {
        setError({ message: err.message })
      } else {
        setError({ message: 'Unable to mark ready' })
      }
      setActivity({ type: 'idle' })
      activityRef.current = { type: 'idle' }
    } finally {
      // Only complete action if we're still in the ready action state
      // (might have changed if another action started)
      const currentActivity = activityRef.current
      if (
        currentActivity.type === 'action' &&
        currentActivity.action === 'ready'
      ) {
        await completeActionAndRefresh()
      }
    }
  }, [
    canMarkReady,
    gameId,
    hasMarkedReady,
    isReadyPending,
    isIdle,
    completeActionAndRefresh,
  ])

  const handleSubmitBid = useCallback(
    async (bid: number) => {
      if (isBidPending || !isIdle) {
        return
      }

      setActivity({ type: 'action', action: 'bid' })

      try {
        await executeApiAction(() => submitBidAction({ gameId, bid }), {
          successMessage: 'Bid submitted',
        })
      } finally {
        await completeActionAndRefresh()
      }
    },
    [gameId, isBidPending, isIdle, executeApiAction, completeActionAndRefresh]
  )

  const handleSelectTrump = useCallback(
    async (trump: Trump) => {
      if (isTrumpPending || !isIdle) {
        return
      }

      setActivity({ type: 'action', action: 'trump' })

      try {
        await executeApiAction(() => selectTrumpAction({ gameId, trump }), {
          successMessage: 'Trump selected',
        })
      } finally {
        await completeActionAndRefresh()
      }
    },
    [gameId, isTrumpPending, isIdle, executeApiAction, completeActionAndRefresh]
  )

  const handlePlayCard = useCallback(
    async (card: string) => {
      if (isPlayPending || !isIdle) {
        return
      }

      setActivity({ type: 'action', action: 'play' })

      try {
        await executeApiAction(() => submitPlayAction({ gameId, card }), {
          successMessage: 'Card played',
        })
      } finally {
        await completeActionAndRefresh()
      }
    },
    [gameId, isPlayPending, isIdle, executeApiAction, completeActionAndRefresh]
  )

  const viewerSeatForInteractions =
    typeof snapshot.viewerSeat === 'number' ? snapshot.viewerSeat : null

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
  const canViewAiManager = viewerIsHost && phase.phase === 'Init'
  const aiControlsEnabled = canViewAiManager && isIdle

  useEffect(() => {
    if (!canViewAiManager) {
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
  }, [canViewAiManager])

  const handleAddAi = useCallback(
    async (selection?: AiSeatSelection) => {
      if (isAiPending || !aiControlsEnabled || !isIdle) {
        return
      }

      setActivity({ type: 'action', action: 'ai' })

      try {
        const registryName =
          selection?.registryName ??
          aiRegistry.find((entry) => entry.name === DEFAULT_AI_NAME)?.name ??
          DEFAULT_AI_NAME
        const registryVersion =
          selection?.registryVersion ??
          aiRegistry.find((entry) => entry.name === registryName)?.version

        await executeApiAction(
          () =>
            addAiSeatAction({
              gameId,
              registryName,
              registryVersion,
              seed: selection?.seed,
            }),
          {
            successMessage: 'AI seat added',
            errorMessage: 'Failed to add AI seat',
          }
        )
      } finally {
        await completeActionAndRefresh()
      }
    },
    [
      aiRegistry,
      aiControlsEnabled,
      gameId,
      isAiPending,
      isIdle,
      executeApiAction,
      completeActionAndRefresh,
    ]
  )

  const handleRemoveAiSeat = useCallback(
    async (seat: Seat) => {
      if (isAiPending || !aiControlsEnabled || !isIdle) {
        return
      }

      setActivity({ type: 'action', action: 'ai' })

      try {
        await executeApiAction(() => removeAiSeatAction({ gameId, seat }), {
          successMessage: 'AI seat removed',
          errorMessage: 'Failed to remove AI seat',
        })
      } finally {
        await completeActionAndRefresh()
      }
    },
    [
      aiControlsEnabled,
      gameId,
      isAiPending,
      isIdle,
      executeApiAction,
      completeActionAndRefresh,
    ]
  )

  const handleUpdateAiSeat = useCallback(
    async (seat: Seat, selection: AiSeatSelection) => {
      if (isAiPending || !aiControlsEnabled || !isIdle) {
        return
      }

      setActivity({ type: 'action', action: 'ai' })

      try {
        await executeApiAction(
          () =>
            updateAiSeatAction({
              gameId,
              seat,
              registryName: selection.registryName,
              registryVersion: selection.registryVersion,
              seed: selection.seed,
            }),
          {
            successMessage: 'AI seat updated',
            errorMessage: 'Failed to update AI seat',
          }
        )
      } finally {
        await completeActionAndRefresh()
      }
    },
    [
      aiControlsEnabled,
      gameId,
      isAiPending,
      isIdle,
      executeApiAction,
      completeActionAndRefresh,
    ]
  )

  const aiSeatState = useMemo(() => {
    if (!canViewAiManager) {
      return undefined
    }

    return {
      totalSeats,
      availableSeats,
      aiSeats,
      isPending: isAiPending || !aiControlsEnabled,
      canAdd: availableSeats > 0 && !isAiRegistryLoading && aiControlsEnabled,
      canRemove: aiSeats > 0 && aiControlsEnabled,
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
    aiControlsEnabled,
    canViewAiManager,
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
        onRefresh={() => void requestRefresh('manual')}
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
      <Toast toast={toast} onClose={hideToast} />
    </>
  )
}
