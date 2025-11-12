import type { Card, Seat, Trump } from '@/lib/game-room/types'

export interface AiSeatSelection {
  registryName: string
  registryVersion?: string
  seed?: number
}

export interface GameRoomStatus {
  lastSyncedAt: string
  isPolling: boolean
}

export interface GameRoomError {
  message: string
  traceId?: string
}

export interface ReadyState {
  canReady: boolean
  isPending: boolean
  hasMarked: boolean
  onReady: () => void
}

export interface BiddingState {
  viewerSeat: Seat
  isPending: boolean
  onSubmit: (bid: number) => Promise<void> | void
}

export interface TrumpState {
  viewerSeat: Seat
  toAct: Seat
  allowedTrumps: Trump[]
  canSelect: boolean
  isPending: boolean
  onSelect?: (trump: Trump) => Promise<void> | void
}

export interface PlayState {
  viewerSeat: Seat
  playable: Card[]
  isPending: boolean
  onPlay: (card: Card) => Promise<void> | void
}

export interface AiSeatRegistryEntry {
  name: string
  version: string
}

export interface AiSeatRegistry {
  entries: AiSeatRegistryEntry[]
  isLoading: boolean
  error?: string | null
  defaultName?: string
}

export interface AiSeatEntry {
  seat: Seat
  name: string
  userId: number | null
  isOccupied: boolean
  isAi: boolean
  isReady: boolean
  aiProfile?: {
    name: string
    version: string
  } | null
}

export interface AiSeatState {
  totalSeats: number
  availableSeats: number
  aiSeats: number
  isPending: boolean
  canAdd: boolean
  canRemove: boolean
  onAdd: (selection?: AiSeatSelection) => Promise<void> | void
  onRemove?: () => Promise<void> | void
  onRemoveSeat?: (seat: Seat) => Promise<void> | void
  onUpdateSeat?: (
    seat: Seat,
    selection: AiSeatSelection
  ) => Promise<void> | void
  registry?: AiSeatRegistry
  seats: AiSeatEntry[]
}
