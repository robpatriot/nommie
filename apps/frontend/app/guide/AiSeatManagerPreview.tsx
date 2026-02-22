'use client'

import { AiSeatManager } from '@/app/game/[gameId]/_components/game-room/AiSeatManager'
import type { AiSeatState } from '@/app/game/[gameId]/_components/game-room-view.types'

const mockAiSeatState: AiSeatState = {
  totalSeats: 4,
  availableSeats: 1,
  aiSeats: 2,
  isPending: false,
  canAdd: true,
  canRemove: true,
  onAdd: () => {},
  onRemoveSeat: () => {},
  onUpdateSeat: () => {},
  registry: {
    entries: [
      { name: 'RandomPlayer', version: '1.0.0' },
      { name: 'Tactician', version: '1.0.0' },
    ],
    isLoading: false,
    defaultName: 'RandomPlayer',
  },
  seats: [
    {
      seat: 0,
      name: 'You',
      userId: 1,
      isOccupied: true,
      isAi: false,
      isReady: true,
      aiProfile: null,
    },
    {
      seat: 1,
      name: 'Bot Bailey',
      userId: null,
      isOccupied: true,
      isAi: true,
      isReady: true,
      aiProfile: { name: 'RandomPlayer', version: '1.0.0' },
    },
    {
      seat: 2,
      name: 'Bot Casey',
      userId: null,
      isOccupied: true,
      isAi: true,
      isReady: true,
      aiProfile: { name: 'Tactician', version: '1.0.0' },
    },
    {
      seat: 3,
      name: '',
      userId: null,
      isOccupied: false,
      isAi: false,
      isReady: false,
      aiProfile: null,
    },
  ],
}

export function AiSeatManagerPreview() {
  return (
    <div className="pointer-events-none mt-6 flex justify-center">
      <div className="w-1/2 select-none opacity-90">
        <AiSeatManager aiState={mockAiSeatState} />
      </div>
    </div>
  )
}
