import { describe, expect, it } from 'vitest'
import {
  getGameRoomSnapshotAction,
  markPlayerReadyAction,
  fetchAiRegistryAction,
} from '@/app/actions/game-room-actions'
import {
  mockGetGameRoomSnapshotAction,
  mockMarkPlayerReadyAction,
  mockFetchAiRegistryAction,
} from '../../setupGameRoomActionsMock'

/**
 * Guard test: Verifies that @/app/actions/game-room-actions is properly mocked
 * by the centralized setup file.
 *
 * This test ensures that the module is mocked at the setup level and prevents
 * accidental reintroduction of per-file mocks.
 */
describe('game-room-actions mock guard', () => {
  it('should have mocked exports from @/app/actions/game-room-actions', () => {
    // Verify that the exported functions exist and are callable
    expect(getGameRoomSnapshotAction).toBeDefined()
    expect(typeof getGameRoomSnapshotAction).toBe('function')

    expect(markPlayerReadyAction).toBeDefined()
    expect(typeof markPlayerReadyAction).toBe('function')

    expect(fetchAiRegistryAction).toBeDefined()
    expect(typeof fetchAiRegistryAction).toBe('function')

    // Verify that calling the exported functions invokes the underlying mock functions
    // by setting up return values and verifying they are called
    mockGetGameRoomSnapshotAction.mockReturnValueOnce({
      kind: 'ok',
      data: null,
    })
    const result1 = getGameRoomSnapshotAction({ gameId: 1 })
    expect(mockGetGameRoomSnapshotAction).toHaveBeenCalledWith({ gameId: 1 })
    expect(result1).toEqual({ kind: 'ok', data: null })

    mockMarkPlayerReadyAction.mockReturnValueOnce({ kind: 'ok' })
    const result2 = markPlayerReadyAction(42, true, 1)
    expect(mockMarkPlayerReadyAction).toHaveBeenCalledWith(42, true, 1)
    expect(result2).toEqual({ kind: 'ok' })

    mockFetchAiRegistryAction.mockReturnValueOnce({ kind: 'ok', data: [] })
    const result3 = fetchAiRegistryAction()
    expect(mockFetchAiRegistryAction).toHaveBeenCalled()
    expect(result3).toEqual({ kind: 'ok', data: [] })
  })
})
