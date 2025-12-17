'use client'

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import {
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
import { useGameSync } from '@/hooks/useGameSync'
import type { Seat, Trump } from '@/lib/game-room/types'

import { GameRoomView, type AiSeatSelection } from './game-room-view'
import type { GameRoomError } from './game-room-view.types'

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
  const {
    snapshot,
    refreshSnapshot,
    syncError,
    isRefreshing: syncIsRefreshing,
  } = useGameSync({ initialData, gameId })
  const [actionError, setActionError] = useState<GameRoomError | null>(null)
  const [hasMarkedReady, setHasMarkedReady] = useState(false)
  const { toasts, showToast, hideToast } = useToast()
  const [aiRegistry, setAiRegistry] = useState<AiRegistryEntryState[]>([])
  const [isAiRegistryLoading, setIsAiRegistryLoading] = useState(false)
  const [aiRegistryError, setAiRegistryError] = useState<string | null>(null)
  const [pendingAction, setPendingAction] = useState<PendingAction>(null)
  const slowSyncToastIdRef = useRef<string | null>(null)
  const combinedError = actionError ?? syncError
  const isReadyPending = pendingAction === 'ready'
  const isBidPending = pendingAction === 'bid'
  const isTrumpPending = pendingAction === 'trump'
  const isPlayPending = pendingAction === 'play'
  const isAiPending = pendingAction === 'ai'

  const phase = snapshot.snapshot.phase
  const phaseName = phase.phase
  const canMarkReady = phaseName === 'Init'

  // Calculate viewer seat once and reuse
  const viewerSeatForInteractions = useMemo(
    () =>
      typeof snapshot.viewerSeat === 'number' ? snapshot.viewerSeat : null,
    [snapshot.viewerSeat]
  )

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
    if (!syncIsRefreshing) {
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
  }, [syncIsRefreshing, showToast, hideToast])

  const executeApiAction = useApiAction({
    showToast,
    // Don't trigger refresh here - action handlers will do it after activity resets
  })

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
        const result = await markPlayerReadyAction(gameId)
        if (result.kind === 'error') {
          setActionError({ message: result.message, traceId: result.traceId })
          return
        }
        setHasMarkedReady(true)
      } catch (err) {
        setActionError({
          message: err instanceof Error ? err.message : 'Unable to mark ready',
        })
      }
    })
  }, [canMarkReady, gameId, hasMarkedReady, isReadyPending, runExclusiveAction])

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
        await executeApiAction(
          () =>
            submitBidAction({
              gameId,
              bid,
              lockVersion: snapshot.lockVersion!,
            }),
          {
            successMessage: 'Bid submitted',
          }
        )
      })
    },
    [
      gameId,
      isBidPending,
      executeApiAction,
      runExclusiveAction,
      snapshot.lockVersion,
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
        await executeApiAction(
          () =>
            selectTrumpAction({
              gameId,
              trump,
              lockVersion: snapshot.lockVersion!,
            }),
          {
            successMessage: 'Trump selected',
          }
        )
      })
    },
    [
      gameId,
      isTrumpPending,
      executeApiAction,
      runExclusiveAction,
      snapshot.lockVersion,
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
        await executeApiAction(
          () =>
            submitPlayAction({
              gameId,
              card,
              lockVersion: snapshot.lockVersion!,
            }),
          {
            successMessage: 'Card played',
          }
        )
      })
    },
    [
      gameId,
      isPlayPending,
      executeApiAction,
      runExclusiveAction,
      snapshot.lockVersion,
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

  const hostSeat: Seat = snapshot.hostSeat
  const viewerIsHost = viewerSeatForInteractions === hostSeat
  const canViewAiManager = viewerIsHost && phase.phase === 'Init'
  const aiControlsEnabled = canViewAiManager

  // AI registry fetch effect: Loads AI registry when AI manager is visible
  // Cleanup: Cancels pending fetch on unmount or when canViewAiManager changes to prevent memory leaks
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
        // Check cancelled flag before updating state to prevent state updates on unmounted component
        if (cancelled) {
          return
        }

        if (result.kind === 'ok') {
          setAiRegistry(result.data)
        } else {
          setAiRegistryError(result.message)
        }
      })
      .catch((err) => {
        // Check cancelled flag before updating state to prevent state updates on unmounted component
        if (cancelled) {
          return
        }

        const message =
          err instanceof Error ? err.message : 'Failed to load AI registry'
        setAiRegistryError(message)
      })
      .finally(() => {
        // Only update loading state if not cancelled to prevent state updates on unmounted component
        if (!cancelled) {
          setIsAiRegistryLoading(false)
        }
      })

    // Cleanup: Set cancelled flag to prevent state updates after component unmounts
    return () => {
      cancelled = true
    }
  }, [canViewAiManager])

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
      })
    },
    [
      aiRegistry,
      aiControlsEnabled,
      gameId,
      isAiPending,
      executeApiAction,
      runExclusiveAction,
    ]
  )

  const handleRemoveAiSeat = useCallback(
    async (seat: Seat) => {
      if (isAiPending || !aiControlsEnabled) {
        return
      }

      await runExclusiveAction('ai', async () => {
        await executeApiAction(() => removeAiSeatAction({ gameId, seat }), {
          successMessage: 'AI seat removed',
          errorMessage: 'Failed to remove AI seat',
        })
      })
    },
    [
      aiControlsEnabled,
      gameId,
      isAiPending,
      executeApiAction,
      runExclusiveAction,
    ]
  )

  const handleUpdateAiSeat = useCallback(
    async (seat: Seat, selection: AiSeatSelection) => {
      if (isAiPending || !aiControlsEnabled) {
        return
      }

      await runExclusiveAction('ai', async () => {
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
      })
    },
    [
      aiControlsEnabled,
      gameId,
      isAiPending,
      executeApiAction,
      runExclusiveAction,
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
        isRefreshing={syncIsRefreshing}
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
