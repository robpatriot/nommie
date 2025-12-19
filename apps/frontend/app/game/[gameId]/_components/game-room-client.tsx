'use client'

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import Toast from '@/components/Toast'
import { useToast } from '@/hooks/useToast'
import { useGameSync } from '@/hooks/useGameSync'
import { useGameRoomSnapshot } from '@/hooks/queries/useGameRoomSnapshot'
import type { Seat, Trump } from '@/lib/game-room/types'

import { GameRoomView, type AiSeatSelection } from './game-room-view'
import type { GameRoomError } from './game-room-view.types'
import { useAiRegistry } from '@/hooks/queries/useAi'
import {
  useMarkPlayerReady,
  useSubmitBid,
  useSelectTrump,
  useSubmitPlay,
  useAddAiSeat,
  useUpdateAiSeat,
  useRemoveAiSeat,
} from '@/hooks/mutations/useGameRoomMutations'
import {
  getGameRoomError,
  toQueryError,
} from '@/lib/queries/query-error-handler'

const DEFAULT_AI_NAME = 'HeuristicV1'

type AiRegistryEntryState = {
  name: string
  version: string
}

type PendingAction = 'ready' | 'bid' | 'trump' | 'play' | 'ai' | null

interface GameRoomClientProps {
  initialData: GameRoomSnapshotPayload
  gameId: number
  requireCardConfirmation?: boolean
}

export function GameRoomClient({
  initialData,
  gameId,
  requireCardConfirmation = true,
}: GameRoomClientProps) {
  // Read snapshot from TanStack Query cache (single source of truth)
  // WebSocket updates will automatically update the cache and trigger re-renders
  const {
    data: snapshot = initialData,
    error: queryError,
    isFetching: isSnapshotFetching,
  } = useGameRoomSnapshot(gameId, {
    initialData,
    etag: initialData.etag,
  })

  // Get WebSocket connection state and refresh function
  const {
    refreshSnapshot,
    syncError,
    isRefreshing: syncIsRefreshing,
  } = useGameSync({ initialData, gameId })

  const [actionError, setActionError] = useState<GameRoomError | null>(null)
  const [hasMarkedReady, setHasMarkedReady] = useState(false)
  const { toasts, showToast, hideToast } = useToast()
  const [pendingAction, setPendingAction] = useState<PendingAction>(null)

  // Combine errors from query, WebSocket, and actions
  const combinedError = actionError ?? syncError ?? getGameRoomError(queryError)

  // Combine loading/refreshing states
  const isRefreshing = syncIsRefreshing || isSnapshotFetching

  // Calculate viewer seat once and reuse
  const viewerSeatForInteractions = useMemo(
    () =>
      typeof snapshot.viewerSeat === 'number' ? snapshot.viewerSeat : null,
    [snapshot.viewerSeat]
  )

  const phase = snapshot.snapshot.phase
  const phaseName = phase.phase
  const canMarkReady = phaseName === 'Init'

  // AI registry query - only enabled when AI manager is visible
  const hostSeat: Seat = snapshot.hostSeat
  const viewerIsHost = viewerSeatForInteractions === hostSeat
  const canViewAiManager = viewerIsHost && phaseName === 'Init'

  const {
    data: aiRegistryData = [],
    isLoading: isAiRegistryLoading,
    error: aiRegistryQueryError,
  } = useAiRegistry(canViewAiManager)

  // Convert query error to string for compatibility
  const aiRegistryError = aiRegistryQueryError
    ? aiRegistryQueryError instanceof Error
      ? aiRegistryQueryError.message
      : 'Failed to load AI registry'
    : null

  // Convert AI registry data to expected format
  const aiRegistry: AiRegistryEntryState[] = aiRegistryData

  // Mutations
  const markPlayerReadyMutation = useMarkPlayerReady()
  const submitBidMutation = useSubmitBid()
  const selectTrumpMutation = useSelectTrump()
  const submitPlayMutation = useSubmitPlay()
  const addAiSeatMutation = useAddAiSeat()
  const updateAiSeatMutation = useUpdateAiSeat()
  const removeAiSeatMutation = useRemoveAiSeat()
  const slowSyncToastIdRef = useRef<string | null>(null)
  const isReadyPending =
    pendingAction === 'ready' || markPlayerReadyMutation.isPending
  const isBidPending = pendingAction === 'bid' || submitBidMutation.isPending
  const isTrumpPending =
    pendingAction === 'trump' || selectTrumpMutation.isPending
  const isPlayPending = pendingAction === 'play' || submitPlayMutation.isPending
  const isAiPending =
    pendingAction === 'ai' ||
    addAiSeatMutation.isPending ||
    updateAiSeatMutation.isPending ||
    removeAiSeatMutation.isPending

  // Initialize hasMarkedReady from snapshot on mount and when snapshot updates
  useEffect(() => {
    if (viewerSeatForInteractions !== null && canMarkReady) {
      const viewerSeatAssignment = snapshot.snapshot.game.seating.find(
        (seat, index) => {
          const seatIndex =
            typeof seat.seat === 'number' && !Number.isNaN(seat.seat)
              ? seat.seat
              : index
          return seatIndex === viewerSeatForInteractions
        }
      )
      if (viewerSeatAssignment) {
        setHasMarkedReady(viewerSeatAssignment.is_ready)
      }
    }
  }, [snapshot.snapshot.game.seating, viewerSeatForInteractions, canMarkReady])

  // Reset hasMarkedReady when phase changes away from Init.
  // Use phase directly (not canMarkReady) to avoid race conditions on rapid phase changes.
  useEffect(() => {
    if (phaseName !== 'Init' && hasMarkedReady) {
      setHasMarkedReady(false)
    }
  }, [phaseName, hasMarkedReady])

  // Show slow sync indicator when refresh takes longer than 1 second
  useEffect(() => {
    if (!isRefreshing) {
      // Hide toast when refresh completes
      if (slowSyncToastIdRef.current) {
        hideToast(slowSyncToastIdRef.current)
        slowSyncToastIdRef.current = null
      }
      return
    }

    const timeoutId = setTimeout(() => {
      const toastId = showToast('Updating game state…', 'warning')
      slowSyncToastIdRef.current = toastId
    }, 1000)

    return () => {
      clearTimeout(timeoutId)
    }
  }, [isRefreshing, showToast, hideToast])

  const finishAction = useCallback(() => {
    setPendingAction(null)
  }, [])

  const runExclusiveAction = useCallback(
    async (
      actionType: Exclude<PendingAction, null>,
      actionFn: () => Promise<void>
    ) => {
      if (pendingAction) {
        return
      }
      setPendingAction(actionType)
      setActionError(null)
      try {
        await actionFn()
      } finally {
        finishAction()
      }
    },
    [finishAction, pendingAction]
  )

  const markReady = useCallback(async () => {
    if (!canMarkReady || isReadyPending || hasMarkedReady) {
      return
    }

    await runExclusiveAction('ready', async () => {
      try {
        await markPlayerReadyMutation.mutateAsync(gameId)
        setHasMarkedReady(true)
      } catch (err) {
        const backendError = toQueryError(err, 'Unable to mark ready')
        setActionError({
          message: backendError.message,
          traceId: backendError.traceId,
        })
        showToast(backendError.message, 'error', backendError)
      }
    })
  }, [
    canMarkReady,
    gameId,
    hasMarkedReady,
    isReadyPending,
    runExclusiveAction,
    markPlayerReadyMutation,
    showToast,
  ])

  const handleSubmitBid = useCallback(
    async (bid: number) => {
      if (isBidPending) {
        return
      }

      await runExclusiveAction('bid', async () => {
        if (snapshot.lockVersion === undefined) {
          setActionError({
            message: 'Lock version is required to submit bid',
          })
          return
        }
        try {
          await submitBidMutation.mutateAsync({
            gameId,
            bid,
            lockVersion: snapshot.lockVersion!,
          })
          showToast('Bid submitted', 'success')
        } catch (err) {
          const backendError = toQueryError(err, 'Failed to submit bid')
          showToast(backendError.message, 'error', backendError)
        }
      })
    },
    [
      gameId,
      isBidPending,
      runExclusiveAction,
      snapshot.lockVersion,
      submitBidMutation,
      showToast,
    ]
  )

  const handleSelectTrump = useCallback(
    async (trump: Trump) => {
      if (isTrumpPending) {
        return
      }

      await runExclusiveAction('trump', async () => {
        if (snapshot.lockVersion === undefined) {
          setActionError({
            message: 'Lock version is required to select trump',
          })
          return
        }
        try {
          await selectTrumpMutation.mutateAsync({
            gameId,
            trump,
            lockVersion: snapshot.lockVersion!,
          })
          showToast('Trump selected', 'success')
        } catch (err) {
          const backendError = toQueryError(err, 'Failed to select trump')
          showToast(backendError.message, 'error', backendError)
        }
      })
    },
    [
      gameId,
      isTrumpPending,
      runExclusiveAction,
      snapshot.lockVersion,
      selectTrumpMutation,
      showToast,
    ]
  )

  const handlePlayCard = useCallback(
    async (card: string) => {
      if (isPlayPending) {
        return
      }

      await runExclusiveAction('play', async () => {
        if (snapshot.lockVersion === undefined) {
          setActionError({
            message: 'Lock version is required to play card',
          })
          return
        }
        try {
          await submitPlayMutation.mutateAsync({
            gameId,
            card,
            lockVersion: snapshot.lockVersion!,
          })
          showToast('Card played', 'success')
        } catch (err) {
          const backendError = toQueryError(err, 'Failed to play card')
          showToast(backendError.message, 'error', backendError)
        }
      })
    },
    [
      gameId,
      isPlayPending,
      runExclusiveAction,
      snapshot.lockVersion,
      submitPlayMutation,
      showToast,
    ]
  )

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
      zeroBidLocked: snapshot.bidConstraints?.zeroBidLocked ?? false,
      onSubmit: handleSubmitBid,
    }
  }, [
    handleSubmitBid,
    isBidPending,
    phase,
    viewerSeatForInteractions,
    snapshot.bidConstraints,
  ])

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
        // Consider both human and AI assignments as occupying the seat
        isOccupied: Boolean(seat.user_id) || seat.is_ai,
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

  // hostSeat, viewerIsHost, and canViewAiManager are already declared above
  const aiControlsEnabled = canViewAiManager

  // AI registry is now managed by useAiRegistry query hook
  // The query is enabled/disabled based on canViewAiManager

  const handleAddAi = useCallback(
    async (selection?: AiSeatSelection) => {
      if (isAiPending || !aiControlsEnabled) {
        return
      }

      await runExclusiveAction('ai', async () => {
        const registryName =
          selection?.registryName ??
          aiRegistry.find((entry) => entry.name === DEFAULT_AI_NAME)?.name ??
          DEFAULT_AI_NAME
        const registryVersion =
          selection?.registryVersion ??
          aiRegistry.find((entry) => entry.name === registryName)?.version

        try {
          await addAiSeatMutation.mutateAsync({
            gameId,
            registryName,
            registryVersion,
            seed: selection?.seed,
          })
          showToast('AI seat added', 'success')
        } catch (err) {
          const backendError = toQueryError(err, 'Failed to add AI seat')
          showToast(backendError.message, 'error', backendError)
        }
      })
    },
    [
      aiRegistry,
      aiControlsEnabled,
      gameId,
      isAiPending,
      runExclusiveAction,
      addAiSeatMutation,
      showToast,
    ]
  )

  const handleRemoveAiSeat = useCallback(
    async (seat: Seat) => {
      if (isAiPending || !aiControlsEnabled) {
        return
      }

      await runExclusiveAction('ai', async () => {
        try {
          await removeAiSeatMutation.mutateAsync({ gameId, seat })
          showToast('AI seat removed', 'success')
        } catch (err) {
          const backendError = toQueryError(err, 'Failed to remove AI seat')
          showToast(backendError.message, 'error', backendError)
        }
      })
    },
    [
      aiControlsEnabled,
      gameId,
      isAiPending,
      runExclusiveAction,
      removeAiSeatMutation,
      showToast,
    ]
  )

  const handleUpdateAiSeat = useCallback(
    async (seat: Seat, selection: AiSeatSelection) => {
      if (isAiPending || !aiControlsEnabled) {
        return
      }

      await runExclusiveAction('ai', async () => {
        try {
          await updateAiSeatMutation.mutateAsync({
            gameId,
            seat,
            registryName: selection.registryName,
            registryVersion: selection.registryVersion,
            seed: selection.seed,
          })
          showToast('AI seat updated', 'success')
        } catch (err) {
          const backendError = toQueryError(err, 'Failed to update AI seat')
          showToast(backendError.message, 'error', backendError)
        }
      })
    },
    [
      aiControlsEnabled,
      gameId,
      isAiPending,
      runExclusiveAction,
      updateAiSeatMutation,
      showToast,
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
        viewerSeat={snapshot.viewerSeat ?? null}
        viewerHand={snapshot.viewerHand}
        onRefresh={() => void refreshSnapshot()}
        isRefreshing={isRefreshing}
        error={combinedError}
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
        requireCardConfirmation={requireCardConfirmation}
      />
      <Toast toasts={toasts} onClose={hideToast} />
    </>
  )
}
