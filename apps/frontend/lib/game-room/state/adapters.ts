import type { GameStateMsg } from '@/lib/game-room/protocol/types'
import type { GameRoomState } from './types'

type AdapterOpts = {
  receivedAt?: string
  source?: 'ws' | 'http'
}

export function gameStateMsgToRoomState(
  msg: GameStateMsg,
  opts: AdapterOpts = {}
): GameRoomState {
  return {
    topic: msg.topic,
    version: msg.version,
    game: msg.game,
    viewer: msg.viewer,
    ...(opts.receivedAt && { receivedAt: opts.receivedAt }),
    ...(opts.source && { source: opts.source }),
  }
}
