/**
 * Utility functions for player name normalization and extraction.
 */

export interface SeatWithDisplayName {
  display_name?: string | null
}

/**
 * Extracts player name from a seat, using a fallback if name is missing.
 *
 * @param seat - Seat object with optional display_name
 * @param index - Zero-based seat index (0-3)
 * @returns Normalized player name
 */
export function extractPlayerName(
  seat: SeatWithDisplayName,
  index: number
): string {
  const name = seat.display_name?.trim()
  if (name && name.length > 0) {
    return name
  }
  return `Seat ${index + 1}`
}

/**
 * Extracts player names from an array of seats.
 * Returns a tuple of exactly 4 names.
 *
 * @param seats - Array of seats with optional display_name
 * @returns Tuple of 4 player names
 */
export function extractPlayerNames(
  seats: SeatWithDisplayName[]
): [string, string, string, string] {
  return seats.map((seat, index) => extractPlayerName(seat, index)) as [
    string,
    string,
    string,
    string,
  ]
}
