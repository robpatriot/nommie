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

import type { GameSnapshot, Seat } from '@/lib/game-room/types'

/**
 * WS topic. Backend uses `kind` (not `type`).
 */
export type Topic = { kind: 'game'; id: number }

export type { GameSnapshot }

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

export type AckCommand = 'subscribe' | 'unsubscribe'

export type AckMsg = {
  type: 'ack'
  command: AckCommand
  topic: Topic
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

export type LongWaitInvalidatedMsg = {
  type: 'long_wait_invalidated'
  game_id: number
}

export type ServerMsg =
  | HelloAckMsg
  | AckMsg
  | ErrorMsg
  | GameStateMsg
  | LongWaitInvalidatedMsg

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
