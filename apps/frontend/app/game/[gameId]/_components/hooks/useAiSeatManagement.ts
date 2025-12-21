import { useCallback, useMemo } from 'react'
import type { Seat } from '@/lib/game-room/types'
import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import { useAiRegistry } from '@/hooks/queries/useAi'
import {
  useAddAiSeat,
  useUpdateAiSeat,
  useRemoveAiSeat,
} from '@/hooks/mutations/useGameRoomMutations'
import { useToast } from '@/hooks/useToast'
import { toQueryError } from '@/lib/queries/query-error-handler'
import type { AiSeatSelection } from '../game-room-view'

const DEFAULT_AI_NAME = 'HeuristicV1'

type AiRegistryEntryState = {
  name: string
  version: string
}

interface UseAiSeatManagementProps {
  gameId: number
  snapshot: GameRoomSnapshotPayload
  canViewAiManager: boolean
}

/**
 * Manages AI seat operations (add, update, remove) and computes AI seat state.
 * Uses TanStack Query mutation state for pending and error handling.
 */
export function useAiSeatManagement({
  gameId,
  snapshot,
  canViewAiManager,
}: UseAiSeatManagementProps) {
  const { showToast } = useToast()

  // AI registry query - only enabled when AI manager is visible
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
  const addAiSeatMutation = useAddAiSeat()
  const updateAiSeatMutation = useUpdateAiSeat()
  const removeAiSeatMutation = useRemoveAiSeat()

  // Combined pending state from all AI mutations
  const isAiPending =
    addAiSeatMutation.isPending ||
    updateAiSeatMutation.isPending ||
    removeAiSeatMutation.isPending

  const aiControlsEnabled = canViewAiManager

  const handleAddAi = useCallback(
    async (selection?: AiSeatSelection) => {
      if (isAiPending || !aiControlsEnabled) {
        return
      }

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
          lockVersion: snapshot.lockVersion,
        })
        showToast('AI seat added', 'success')
      } catch (err) {
        const backendError = toQueryError(err, 'Failed to add AI seat')
        showToast(backendError.message, 'error', backendError)
      }
    },
    [
      aiRegistry,
      aiControlsEnabled,
      gameId,
      isAiPending,
      addAiSeatMutation,
      showToast,
      snapshot.lockVersion,
    ]
  )

  const handleRemoveAiSeat = useCallback(
    async (seat: Seat) => {
      if (isAiPending || !aiControlsEnabled) {
        return
      }

      try {
        await removeAiSeatMutation.mutateAsync({
          gameId,
          seat,
          lockVersion: snapshot.lockVersion,
        })
        showToast('AI seat removed', 'success')
      } catch (err) {
        const backendError = toQueryError(err, 'Failed to remove AI seat')
        showToast(backendError.message, 'error', backendError)
      }
    },
    [
      aiControlsEnabled,
      gameId,
      isAiPending,
      removeAiSeatMutation,
      showToast,
      snapshot.lockVersion,
    ]
  )

  const handleUpdateAiSeat = useCallback(
    async (seat: Seat, selection: AiSeatSelection) => {
      if (isAiPending || !aiControlsEnabled) {
        return
      }

      try {
        await updateAiSeatMutation.mutateAsync({
          gameId,
          seat,
          registryName: selection.registryName,
          registryVersion: selection.registryVersion,
          seed: selection.seed,
          lockVersion: snapshot.lockVersion,
        })
        showToast('AI seat updated', 'success')
      } catch (err) {
        const backendError = toQueryError(err, 'Failed to update AI seat')
        showToast(backendError.message, 'error', backendError)
      }
    },
    [
      aiControlsEnabled,
      gameId,
      isAiPending,
      updateAiSeatMutation,
      showToast,
      snapshot.lockVersion,
    ]
  )

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

  const aiSeatState = useMemo(() => {
    if (!canViewAiManager) {
      return undefined
    }

    return {
      totalSeats,
      availableSeats,
      aiSeats,
      isPending: isAiPending,
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

  return {
    aiSeatState,
  }
}
