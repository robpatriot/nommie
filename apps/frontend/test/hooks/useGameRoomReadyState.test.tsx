import { describe, expect, it } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useGameRoomReadyState } from '@/app/game/[gameId]/_components/hooks/useGameRoomReadyState'
import type { GameRoomSnapshotPayload } from '@/app/actions/game-room-actions'
import { initSnapshotFixture } from '../mocks/game-snapshot'

function createSnapshotData(
  overrides?: Partial<GameRoomSnapshotPayload>
): GameRoomSnapshotPayload {
  return {
    snapshot: initSnapshotFixture,
    etag: '"game-42-v1"',
    version: 1,
    playerNames: ['Alex', 'Bailey', 'Casey', 'Dakota'],
    viewerSeat: 0,
    viewerHand: [],
    timestamp: new Date().toISOString(),
    hostSeat: 0,
    bidConstraints: null,
    ...overrides,
  }
}

describe('useGameRoomReadyState', () => {
  describe('Initialization', () => {
    it('initializes with hasMarkedReady from snapshot', () => {
      const snapshot = createSnapshotData({
        snapshot: {
          ...initSnapshotFixture,
          game: {
            ...initSnapshotFixture.game,
            seating: [
              {
                seat: 0,
                user_id: 101,
                display_name: 'Alex',
                is_ai: false,
                is_ready: true,
              },
              {
                seat: 1,
                user_id: 202,
                display_name: 'Bailey',
                is_ai: false,
                is_ready: false,
              },
              {
                seat: 2,
                user_id: 303,
                display_name: 'Casey',
                is_ai: false,
                is_ready: false,
              },
              {
                seat: 3,
                user_id: 404,
                display_name: 'Dakota',
                is_ai: false,
                is_ready: false,
              },
            ],
          },
        },
      })

      const { result } = renderHook(() =>
        useGameRoomReadyState(snapshot, 0, 'Init')
      )

      expect(result.current.hasMarkedReady).toBe(true)
      expect(result.current.canMarkReady).toBe(true)
    })

    it('initializes with false when snapshot shows not ready', () => {
      const snapshot = createSnapshotData({
        snapshot: {
          ...initSnapshotFixture,
          game: {
            ...initSnapshotFixture.game,
            seating: [
              {
                seat: 0,
                user_id: 101,
                display_name: 'Alex',
                is_ai: false,
                is_ready: false,
              },
              {
                seat: 1,
                user_id: 202,
                display_name: 'Bailey',
                is_ai: false,
                is_ready: false,
              },
              {
                seat: 2,
                user_id: 303,
                display_name: 'Casey',
                is_ai: false,
                is_ready: false,
              },
              {
                seat: 3,
                user_id: 404,
                display_name: 'Dakota',
                is_ai: false,
                is_ready: false,
              },
            ],
          },
        },
      })

      const { result } = renderHook(() =>
        useGameRoomReadyState(snapshot, 0, 'Init')
      )

      expect(result.current.hasMarkedReady).toBe(false)
      expect(result.current.canMarkReady).toBe(true)
    })

    it('does not sync when viewerSeatForInteractions is null', () => {
      const snapshot = createSnapshotData()

      const { result } = renderHook(() =>
        useGameRoomReadyState(snapshot, null, 'Init')
      )

      expect(result.current.hasMarkedReady).toBe(false)
      expect(result.current.canMarkReady).toBe(true)
    })

    it('does not sync when not in Init phase', () => {
      const snapshot = createSnapshotData({
        snapshot: {
          ...initSnapshotFixture,
          game: {
            ...initSnapshotFixture.game,
            seating: [
              {
                seat: 0,
                user_id: 101,
                display_name: 'Alex',
                is_ai: false,
                is_ready: true,
              },
              {
                seat: 1,
                user_id: 202,
                display_name: 'Bailey',
                is_ai: false,
                is_ready: false,
              },
              {
                seat: 2,
                user_id: 303,
                display_name: 'Casey',
                is_ai: false,
                is_ready: false,
              },
              {
                seat: 3,
                user_id: 404,
                display_name: 'Dakota',
                is_ai: false,
                is_ready: false,
              },
            ],
          },
        },
      })

      const { result } = renderHook(() =>
        useGameRoomReadyState(snapshot, 0, 'Bidding')
      )

      expect(result.current.hasMarkedReady).toBe(false)
      expect(result.current.canMarkReady).toBe(false)
    })
  })

  describe('canMarkReady', () => {
    it('returns true when phase is Init', () => {
      const snapshot = createSnapshotData()

      const { result } = renderHook(() =>
        useGameRoomReadyState(snapshot, 0, 'Init')
      )

      expect(result.current.canMarkReady).toBe(true)
    })

    it('returns false when phase is not Init', () => {
      const snapshot = createSnapshotData()

      const { result } = renderHook(() =>
        useGameRoomReadyState(snapshot, 0, 'Bidding')
      )

      expect(result.current.canMarkReady).toBe(false)
    })
  })

  describe('Phase Change Reset', () => {
    it('resets hasMarkedReady when phase changes from Init to Bidding', () => {
      const snapshot = createSnapshotData()

      const { result, rerender } = renderHook(
        ({ phaseName }) => useGameRoomReadyState(snapshot, 0, phaseName),
        {
          initialProps: { phaseName: 'Init' },
        }
      )

      // Set ready state manually
      act(() => {
        result.current.setHasMarkedReady(true)
      })

      expect(result.current.hasMarkedReady).toBe(true)

      // Change phase
      rerender({ phaseName: 'Bidding' })

      expect(result.current.hasMarkedReady).toBe(false)
    })

    it('does not reset when phase stays in Init', () => {
      const snapshot = createSnapshotData()

      const { result } = renderHook(() =>
        useGameRoomReadyState(snapshot, 0, 'Init')
      )

      act(() => {
        result.current.setHasMarkedReady(true)
      })

      expect(result.current.hasMarkedReady).toBe(true)

      // Snapshot updates but phase stays Init
      const updatedSnapshot = createSnapshotData({
        snapshot: {
          ...initSnapshotFixture,
          game: {
            ...initSnapshotFixture.game,
            seating: [
              {
                seat: 0,
                user_id: 101,
                display_name: 'Alex',
                is_ai: false,
                is_ready: true,
              },
              {
                seat: 1,
                user_id: 202,
                display_name: 'Bailey',
                is_ai: false,
                is_ready: false,
              },
              {
                seat: 2,
                user_id: 303,
                display_name: 'Casey',
                is_ai: false,
                is_ready: false,
              },
              {
                seat: 3,
                user_id: 404,
                display_name: 'Dakota',
                is_ai: false,
                is_ready: false,
              },
            ],
          },
        },
      })

      const { result: result2 } = renderHook(() =>
        useGameRoomReadyState(updatedSnapshot, 0, 'Init')
      )

      // Should sync from snapshot, not reset
      expect(result2.current.hasMarkedReady).toBe(true)
    })
  })

  describe('Snapshot Sync', () => {
    it('updates hasMarkedReady when snapshot changes', () => {
      const initialSnapshot = createSnapshotData({
        snapshot: {
          ...initSnapshotFixture,
          game: {
            ...initSnapshotFixture.game,
            seating: [
              {
                seat: 0,
                user_id: 101,
                display_name: 'Alex',
                is_ai: false,
                is_ready: false,
              },
              {
                seat: 1,
                user_id: 202,
                display_name: 'Bailey',
                is_ai: false,
                is_ready: false,
              },
              {
                seat: 2,
                user_id: 303,
                display_name: 'Casey',
                is_ai: false,
                is_ready: false,
              },
              {
                seat: 3,
                user_id: 404,
                display_name: 'Dakota',
                is_ai: false,
                is_ready: false,
              },
            ],
          },
        },
      })

      const { result, rerender } = renderHook(
        ({ snapshot }) => useGameRoomReadyState(snapshot, 0, 'Init'),
        {
          initialProps: { snapshot: initialSnapshot },
        }
      )

      expect(result.current.hasMarkedReady).toBe(false)

      // Update snapshot with ready state
      const updatedSnapshot = createSnapshotData({
        snapshot: {
          ...initSnapshotFixture,
          game: {
            ...initSnapshotFixture.game,
            seating: [
              {
                seat: 0,
                user_id: 101,
                display_name: 'Alex',
                is_ai: false,
                is_ready: true,
              },
              {
                seat: 1,
                user_id: 202,
                display_name: 'Bailey',
                is_ai: false,
                is_ready: false,
              },
              {
                seat: 2,
                user_id: 303,
                display_name: 'Casey',
                is_ai: false,
                is_ready: false,
              },
              {
                seat: 3,
                user_id: 404,
                display_name: 'Dakota',
                is_ai: false,
                is_ready: false,
              },
            ],
          },
        },
      })

      rerender({ snapshot: updatedSnapshot })

      expect(result.current.hasMarkedReady).toBe(true)
    })

    it('does not overwrite optimistic updates during mutations', () => {
      const snapshot = createSnapshotData({
        snapshot: {
          ...initSnapshotFixture,
          game: {
            ...initSnapshotFixture.game,
            seating: [
              {
                seat: 0,
                user_id: 101,
                display_name: 'Alex',
                is_ai: false,
                is_ready: false,
              },
              {
                seat: 1,
                user_id: 202,
                display_name: 'Bailey',
                is_ai: false,
                is_ready: false,
              },
              {
                seat: 2,
                user_id: 303,
                display_name: 'Casey',
                is_ai: false,
                is_ready: false,
              },
              {
                seat: 3,
                user_id: 404,
                display_name: 'Dakota',
                is_ai: false,
                is_ready: false,
              },
            ],
          },
        },
      })

      const { result, rerender } = renderHook(
        ({ snapshot }) => useGameRoomReadyState(snapshot, 0, 'Init'),
        {
          initialProps: { snapshot },
        }
      )

      // Optimistically set ready
      act(() => {
        result.current.setHasMarkedReady(true)
      })

      expect(result.current.hasMarkedReady).toBe(true)

      // Snapshot still shows false (hasn't updated yet)
      rerender({ snapshot })

      // Should not overwrite optimistic update
      expect(result.current.hasMarkedReady).toBe(true)
    })
  })

  describe('setHasMarkedReady', () => {
    it('allows manual setting of hasMarkedReady', () => {
      const snapshot = createSnapshotData()

      const { result } = renderHook(() =>
        useGameRoomReadyState(snapshot, 0, 'Init')
      )

      expect(result.current.hasMarkedReady).toBe(false)

      act(() => {
        result.current.setHasMarkedReady(true)
      })

      expect(result.current.hasMarkedReady).toBe(true)

      act(() => {
        result.current.setHasMarkedReady(false)
      })

      expect(result.current.hasMarkedReady).toBe(false)
    })
  })
})
