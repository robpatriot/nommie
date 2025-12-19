import { useEffect, useState } from 'react'
import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'

/**
 * Manages the ready state for the viewer in the game room.
 * Syncs with the snapshot data and resets when phase changes away from Init.
 */
export function useGameRoomReadyState(
  snapshot: GameRoomSnapshotPayload,
  viewerSeatForInteractions: number | null,
  phaseName: string
) {
  const [hasMarkedReady, setHasMarkedReady] = useState(false)
  const canMarkReady = phaseName === 'Init'

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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [phaseName])

  return {
    hasMarkedReady,
    setHasMarkedReady,
    canMarkReady,
  }
}
