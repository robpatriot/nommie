import { useEffect, useRef, useState } from 'react'
import type { GameRoomState } from '@/lib/game-room/state'
import { selectSnapshot } from '@/lib/game-room/state'

/**
 * Manages the ready state for the viewer in the game room.
 * Syncs with the state data and resets when phase changes away from Init.
 */
export function useGameRoomReadyState(
  state: GameRoomState,
  viewerSeatForInteractions: number | null,
  phaseName: string
) {
  const [hasMarkedReady, setHasMarkedReady] = useState(false)
  const canMarkReady =
    phaseName === 'Init' && viewerSeatForInteractions !== null
  const lastSyncedReadyState = useRef<boolean | null>(null)

  const seating = selectSnapshot(state).game.seating

  useEffect(() => {
    if (viewerSeatForInteractions !== null && canMarkReady) {
      const viewerSeatAssignment = seating.find((seat, index) => {
        const seatIndex =
          typeof seat.seat === 'number' && !Number.isNaN(seat.seat)
            ? seat.seat
            : index
        return seatIndex === viewerSeatForInteractions
      })
      if (viewerSeatAssignment) {
        const stateReadyState = viewerSeatAssignment.is_ready
        // Only update if the state value differs from what we last synced
        // This prevents overwriting optimistic updates during mutations
        if (lastSyncedReadyState.current !== stateReadyState) {
          lastSyncedReadyState.current = stateReadyState
          setHasMarkedReady(stateReadyState)
        }
      }
    }
  }, [seating, viewerSeatForInteractions, canMarkReady])

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
