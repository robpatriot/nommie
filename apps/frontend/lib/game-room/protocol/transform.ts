import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import type { Card, Seat } from '@/lib/game-room/types'
import type { BidConstraints } from '@/lib/game-room/types'
import type { GameStateMsg } from '@/lib/game-room/protocol/types'
import { extractPlayerNames } from '@/utils/player-names'
import { DEFAULT_VIEWER_SEAT } from '@/lib/game-room/constants'
import { isValidSeat } from '@/utils/seat-validation'

type TransformOpts = {
  /**
   * Prefer using an HTTP ETag if available (authoritative for HTTP caching).
   * WS does not provide an ETag.
   */
  etag?: string
  /**
   * Override timestamp (useful in tests); defaults to now.
   */
  timestamp?: string
  /**
   * If no etag is provided (e.g. WS), callers may provide a builder.
   */
  buildEtag?: (version: number) => string
}

export function gameStateMsgToSnapshotPayload(
  message: GameStateMsg,
  opts: TransformOpts = {}
): GameRoomSnapshotPayload {
  const version = message.version

  const viewerSeat: Seat | null =
    typeof message.viewer.seat === 'number' && isValidSeat(message.viewer.seat)
      ? (message.viewer.seat as Seat)
      : null

  const bidConstraints: BidConstraints | null = message.viewer.bidConstraints
    ? {
        zeroBidLocked: Boolean(message.viewer.bidConstraints.zeroBidLocked),
      }
    : null

  const normalizedViewerHand: Card[] =
    Array.isArray(message.viewer.hand) &&
    message.viewer.hand.every((c) => typeof c === 'string')
      ? (message.viewer.hand as Card[])
      : []

  const playerNames = extractPlayerNames(message.game.game.seating)

  const timestamp = opts.timestamp ?? new Date().toISOString()

  const etag =
    opts.etag ??
    (typeof opts.buildEtag === 'function' ? opts.buildEtag(version) : undefined)

  const hostSeat = (message.game.game.host_seat ?? DEFAULT_VIEWER_SEAT) as Seat

  return {
    snapshot: message.game,
    playerNames,
    viewerSeat,
    viewerHand: normalizedViewerHand,
    timestamp,
    hostSeat,
    bidConstraints,
    version,
    etag,
  }
}
