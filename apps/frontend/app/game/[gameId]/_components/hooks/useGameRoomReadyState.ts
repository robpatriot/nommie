import { useEffect, useRef, useState } from 'react'
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
  // Only players (with a seat) can mark ready, not spectators
  const canMarkReady =
    phaseName === 'Init' && viewerSeatForInteractions !== null
  // Track the last synced snapshot value to avoid unnecessary updates
  const lastSyncedReadyState = useRef<boolean | null>(null)

  // Initialize hasMarkedReady from snapshot on mount and when snapshot updates
  // Only sync when the actual is_ready value in the snapshot differs from what we last synced
  // to avoid overwriting optimistic updates during mutations
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
        const snapshotReadyState = viewerSeatAssignment.is_ready
        // Only update if the snapshot value differs from what we last synced
        // This prevents overwriting optimistic updates during mutations
        if (lastSyncedReadyState.current !== snapshotReadyState) {
          lastSyncedReadyState.current = snapshotReadyState
          setHasMarkedReady(snapshotReadyState)
        }
      }
    }
  }, [snapshot.snapshot.game.seating, viewerSeatForInteractions, canMarkReady])

  // Reset hasMarkedReady when phase changes away from Init.
  // Use phase directly (not canMarkReady) to avoid race conditions on rapid phase changes.
  useEffect(() => {
    if (phaseName !== 'Init' && hasMarkedReady) {
      setHasMarkedReady(false)
      lastSyncedReadyState.current = null
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [phaseName])

  return {
    hasMarkedReady,
    setHasMarkedReady,
    canMarkReady,
  }
}
