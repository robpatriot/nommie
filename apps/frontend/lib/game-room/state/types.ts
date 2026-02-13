import type { GameStateMsg } from '@/lib/game-room/protocol/types'

/**
 * Frontend cache model for game room state.
 * Thin wrapper derived from GameStateMsg; backend is authoritative.
 */
export type GameRoomState = Omit<GameStateMsg, 'type'> & {
  /** Liveness metadata; may be refreshed on keepalive. Never use for correctness/ordering. */
  receivedAt?: string
  source?: 'ws' | 'http' | 'optimistic'
  /** HTTP ETag for conditional requests; not present for WS updates */
  etag?: string
}
