import { describe, expect, it } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useGameRoomReadyState } from '@/app/game/[gameId]/_components/hooks/useGameRoomReadyState'
import type { GameRoomState } from '@/lib/game-room/state'
import type { Seat } from '@/lib/game-room/types'
import { gameStateMsgToRoomState } from '@/lib/game-room/state'
import { initSnapshotFixture } from '../mocks/game-snapshot'

function createState(
  snapshotOverrides?: Partial<typeof initSnapshotFixture>
): GameRoomState {
  const snapshot = snapshotOverrides
    ? { ...initSnapshotFixture, ...snapshotOverrides }
    : initSnapshotFixture
  const msg = {
    type: 'game_state' as const,
    topic: { kind: 'game' as const, id: 42 },
    version: 1,
    game: snapshot,
    viewer: { seat: 0 as Seat, hand: [], bidConstraints: null },
  }
  const state = gameStateMsgToRoomState(msg, { source: 'http' })
  return { ...state, etag: '"game-42-v1"' }
}

describe('useGameRoomReadyState', () => {
  describe('Initialization', () => {
    it('initializes with hasMarkedReady from snapshot', () => {
      const state = createState({
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
      })

      const { result } = renderHook(() =>
        useGameRoomReadyState(state, 0, 'Init')
      )

      expect(result.current.hasMarkedReady).toBe(true)
      expect(result.current.canMarkReady).toBe(true)
    })

    it('initializes with false when snapshot shows not ready', () => {
      const state = createState({
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
      })

      const { result } = renderHook(() =>
        useGameRoomReadyState(state, 0, 'Init')
      )

      expect(result.current.hasMarkedReady).toBe(false)
      expect(result.current.canMarkReady).toBe(true)
    })

    it('does not sync when viewerSeatForInteractions is null', () => {
      const state = createState()

      const { result } = renderHook(() =>
        useGameRoomReadyState(state, null, 'Init')
      )

      expect(result.current.hasMarkedReady).toBe(false)
      // Spectators (null viewerSeat) cannot mark ready
      expect(result.current.canMarkReady).toBe(false)
    })

    it('does not sync when not in Init phase', () => {
      const state = createState({
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
      })

      const { result } = renderHook(() =>
        useGameRoomReadyState(state, 0, 'Bidding')
      )

      expect(result.current.hasMarkedReady).toBe(false)
      expect(result.current.canMarkReady).toBe(false)
    })
  })

  describe('canMarkReady', () => {
    it('returns true when phase is Init', () => {
      const state = createState()

      const { result } = renderHook(() =>
        useGameRoomReadyState(state, 0, 'Init')
      )

      expect(result.current.canMarkReady).toBe(true)
    })

    it('returns false when phase is not Init', () => {
      const state = createState()

      const { result } = renderHook(() =>
        useGameRoomReadyState(state, 0, 'Bidding')
      )

      expect(result.current.canMarkReady).toBe(false)
    })
  })

  describe('Phase Change Reset', () => {
    it('resets hasMarkedReady when phase changes from Init to Bidding', () => {
      const state = createState()

      const { result, rerender } = renderHook(
        ({ phaseName }) => useGameRoomReadyState(state, 0, phaseName),
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
      const state = createState()

      const { result } = renderHook(() =>
        useGameRoomReadyState(state, 0, 'Init')
      )

      act(() => {
        result.current.setHasMarkedReady(true)
      })

      expect(result.current.hasMarkedReady).toBe(true)

      // Snapshot updates but phase stays Init
      const updatedState = createState({
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
      })

      const { result: result2 } = renderHook(() =>
        useGameRoomReadyState(updatedState, 0, 'Init')
      )

      // Should sync from snapshot, not reset
      expect(result2.current.hasMarkedReady).toBe(true)
    })
  })

  describe('Snapshot Sync', () => {
    it('updates hasMarkedReady when snapshot changes', () => {
      const initialState = createState({
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
      })

      const { result, rerender } = renderHook(
        ({ state }) => useGameRoomReadyState(state, 0, 'Init'),
        {
          initialProps: { state: initialState },
        }
      )

      expect(result.current.hasMarkedReady).toBe(false)

      // Update state with ready state
      const updatedState = createState({
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
      })

      rerender({ state: updatedState })

      expect(result.current.hasMarkedReady).toBe(true)
    })

    it('does not overwrite optimistic updates during mutations', () => {
      const state = createState({
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
      })

      const { result, rerender } = renderHook(
        ({ state: s }) => useGameRoomReadyState(s, 0, 'Init'),
        {
          initialProps: { state },
        }
      )

      act(() => {
        result.current.setHasMarkedReady(true)
      })

      expect(result.current.hasMarkedReady).toBe(true)

      rerender({ state })

      // Should not overwrite optimistic update
      expect(result.current.hasMarkedReady).toBe(true)
    })
  })

  describe('setHasMarkedReady', () => {
    it('allows manual setting of hasMarkedReady', () => {
      const state = createState()

      const { result } = renderHook(() =>
        useGameRoomReadyState(state, 0, 'Init')
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
