// apps/frontend/lib/game-room/protocol/types.ts
// WebSocket protocol types for the Game Room.
// Backend is the source of truth. No shims.
//
// Locked server->client message:
//   { type: "game_state", topic: { kind: "game", id }, version, game, viewer }
//
// Locked client->server messages:
//   { type: "hello", protocol: 1 }
//   { type: "subscribe", topic: { kind: "game", id } }
//   { type: "unsubscribe", topic: { kind: "game", id } } // supported by backend

import type { Seat } from '@/lib/game-room/types'
import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'

/**
 * WS topic. Backend uses `kind` (not `type`).
 */
export type Topic = { kind: 'game'; id: number }

/**
 * Snapshot DTO carried on the wire for `game_state`.
 * We derive from the HTTP payload to avoid duplicating snapshot shapes.
 */
export type GameSnapshot = GameRoomSnapshotPayload['snapshot']

/**
 * Bid constraints sent to the viewer.
 * Keep narrow/explicit; expand only when backend adds fields.
 */
export type BidConstraints = {
  zeroBidLocked: boolean
}

/**
 * Viewer-relative state for a single game. Sent on every `game_state`.
 */
export type ViewerState = {
  seat: Seat | null
  hand: string[]
  bidConstraints: BidConstraints | null
}

/**
 * Server -> Client messages
 */
export type HelloAckMsg = {
  type: 'hello_ack'
  protocol: number
  user_id: number
}

export type AckMsg = {
  type: 'ack'
  message: string
}

export type ErrorMsg = {
  type: 'error'
  code: 'bad_protocol' | 'bad_topic' | 'bad_request' | 'forbidden'
  message: string
}

export type GameStateMsg = {
  type: 'game_state'
  topic: Topic
  version: number
  game: GameSnapshot
  viewer: ViewerState
}

export type ServerMsg = HelloAckMsg | AckMsg | ErrorMsg | GameStateMsg

/**
 * Client -> Server messages
 */
export type HelloMsg = {
  type: 'hello'
  protocol: number
}

export type SubscribeMsg = {
  type: 'subscribe'
  topic: Topic
}

export type UnsubscribeMsg = {
  type: 'unsubscribe'
  topic: Topic
}

export type ClientMsg = HelloMsg | SubscribeMsg | UnsubscribeMsg

/**
 * Lightweight runtime guards (useful in hooks/tests).
 *
 * Note: backend supports more message types than the subset we model here.
 * These guards intentionally validate only the minimal envelope shape.
 */
export type WireMsg = { type: string; [key: string]: unknown }

export function isWireMsg(value: unknown): value is WireMsg {
  return (
    typeof value === 'object' &&
    value !== null &&
    typeof (value as { type?: unknown }).type === 'string'
  )
}

export function isGameStateMsg(msg: WireMsg): msg is GameStateMsg {
  return msg.type === 'game_state'
}
