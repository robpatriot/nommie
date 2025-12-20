import { vi } from 'vitest'

/**
 * Centralized mock for @/app/actions/game-room-actions
 *
 * This is the single authoritative place where @/app/actions/game-room-actions
 * is mocked. Individual test files must NOT mock this module directly.
 *
 * By registering the mock in a setup file (loaded before setupTests.ts), we
 * ensure correct import order and avoid hoisting issues that can occur when
 * mocks are defined in test files.
 */
// Hoisted mock functions for game-room-actions
// Using vi.hoisted() ensures these are available when the mock factory runs
const {
  mockGetGameRoomSnapshotAction,
  mockMarkPlayerReadyAction,
  mockSubmitBidAction,
  mockSelectTrumpAction,
  mockSubmitPlayAction,
  mockAddAiSeatAction,
  mockUpdateAiSeatAction,
  mockRemoveAiSeatAction,
  mockFetchAiRegistryAction,
} = vi.hoisted(() => ({
  mockGetGameRoomSnapshotAction: vi.fn(),
  mockMarkPlayerReadyAction: vi.fn(),
  mockSubmitBidAction: vi.fn(),
  mockSelectTrumpAction: vi.fn(),
  mockSubmitPlayAction: vi.fn(),
  mockAddAiSeatAction: vi.fn(),
  mockUpdateAiSeatAction: vi.fn(),
  mockRemoveAiSeatAction: vi.fn(),
  mockFetchAiRegistryAction: vi.fn(),
}))

// Export the mock functions so test files can configure them
export {
  mockGetGameRoomSnapshotAction,
  mockMarkPlayerReadyAction,
  mockSubmitBidAction,
  mockSelectTrumpAction,
  mockSubmitPlayAction,
  mockAddAiSeatAction,
  mockUpdateAiSeatAction,
  mockRemoveAiSeatAction,
  mockFetchAiRegistryAction,
}

// Register the mock for @/app/actions/game-room-actions
// This must be done in a setup file so it runs before any test files import the module
vi.mock('@/app/actions/game-room-actions', () => ({
  getGameRoomSnapshotAction: (request: unknown) =>
    mockGetGameRoomSnapshotAction(request),
  markPlayerReadyAction: (gameId: number) => mockMarkPlayerReadyAction(gameId),
  submitBidAction: (request: unknown) => mockSubmitBidAction(request),
  selectTrumpAction: (request: unknown) => mockSelectTrumpAction(request),
  submitPlayAction: (request: unknown) => mockSubmitPlayAction(request),
  addAiSeatAction: (request: unknown) => mockAddAiSeatAction(request),
  updateAiSeatAction: (request: unknown) => mockUpdateAiSeatAction(request),
  removeAiSeatAction: (request: unknown) => mockRemoveAiSeatAction(request),
  fetchAiRegistryAction: () => mockFetchAiRegistryAction(),
}))
