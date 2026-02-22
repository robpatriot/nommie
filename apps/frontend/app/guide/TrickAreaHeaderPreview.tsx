'use client'

import { TrickAreaHeader } from '@/app/game/[gameId]/_components/game-room/TrickAreaHeader'
import { cn } from '@/lib/cn'

const MOBILE_TOP_PADDING = 8
const HEADER_HEIGHT = 28

export function TrickAreaHeaderPreview() {
  return (
    <div
      className={cn(
        'oldtime-trick-bg relative overflow-hidden rounded-[32px] border border-border/60 bg-overlay/25 px-4 shadow-elevated backdrop-blur'
      )}
      style={{
        height: `${MOBILE_TOP_PADDING + HEADER_HEIGHT + 8}px`,
        paddingTop: `${MOBILE_TOP_PADDING}px`,
      }}
    >
      <div className="pointer-events-none">
        <TrickAreaHeader
          trump="HEARTS"
          totalBids={12}
          handSize={13}
          alwaysShow
        />
      </div>
    </div>
  )
}
