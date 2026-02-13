import { useMemo } from 'react'
import type { Seat, Trump } from '@/lib/game-room/types'
import type { GameSnapshot } from '@/lib/game-room/types'
import {
  isBiddingPhase,
  isTrumpSelectPhase,
  isTrickPhase,
} from '../game-room/phase-helpers'

interface UseGameRoomControlsProps {
  phase: GameSnapshot['phase']
  viewerSeatForInteractions: Seat | null
  bidConstraints: { zeroBidLocked?: boolean } | null | undefined
  handleSubmitBid: (bid: number) => Promise<void>
  handleSelectTrump: (trump: Trump) => Promise<void>
  handlePlayCard: (card: string) => Promise<void>
  isBidPending: boolean
  isTrumpPending: boolean
  isPlayPending: boolean
}

/**
 * Computes control objects for bidding, trump selection, and card play phases.
 * Returns undefined when controls are not applicable for the current phase.
 */
export function useGameRoomControls({
  phase,
  viewerSeatForInteractions,
  bidConstraints,
  handleSubmitBid,
  handleSelectTrump,
  handlePlayCard,
  isBidPending,
  isTrumpPending,
  isPlayPending,
}: UseGameRoomControlsProps) {
  const biddingControls = useMemo(() => {
    if (
      !isBiddingPhase(phase) ||
      viewerSeatForInteractions === null ||
      phase.data.bids[viewerSeatForInteractions] !== null
    ) {
      return undefined
    }

    return {
      viewerSeat: viewerSeatForInteractions,
      isPending: isBidPending,
      zeroBidLocked: bidConstraints?.zeroBidLocked ?? false,
      onSubmit: handleSubmitBid,
    }
  }, [
    handleSubmitBid,
    isBidPending,
    phase,
    viewerSeatForInteractions,
    bidConstraints,
  ])

  const trumpControls = useMemo(() => {
    if (!isTrumpSelectPhase(phase)) {
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
    if (!isTrickPhase(phase)) {
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

  return {
    biddingControls,
    trumpControls,
    playControls,
  }
}
