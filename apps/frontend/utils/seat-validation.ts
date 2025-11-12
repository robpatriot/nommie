/**
 * Utility functions for seat validation.
 */

import type { Seat } from '@/lib/game-room/types'

/**
 * Validates that a seat number is within valid range (0-3).
 *
 * @param seat - Seat number to validate
 * @returns True if seat is valid, false otherwise
 */
export function isValidSeat(seat: number): seat is Seat {
  return (
    Number.isFinite(seat) &&
    !Number.isNaN(seat) &&
    seat >= 0 &&
    seat <= 3 &&
    Number.isInteger(seat)
  )
}

/**
 * Validates a seat number and returns an error message if invalid.
 *
 * @param seat - Seat number to validate
 * @param required - Whether the seat is required (default: false)
 * @returns Error message if invalid, null if valid
 */
export function validateSeat(
  seat: number | undefined,
  required: boolean = false
): string | null {
  if (required && seat === undefined) {
    return 'Seat is required'
  }

  if (seat === undefined) {
    return null // Optional seat is valid if undefined
  }

  if (!isValidSeat(seat)) {
    return 'Seat must be between 0 and 3'
  }

  return null
}

/**
 * Validates a seat number and throws an error if invalid.
 * Useful for server actions that need to validate before processing.
 *
 * @param seat - Seat number to validate
 * @param required - Whether the seat is required (default: false)
 * @throws Error if seat is invalid
 */
export function requireValidSeat(
  seat: number | undefined,
  required: boolean = false
): void {
  const error = validateSeat(seat, required)
  if (error) {
    throw new Error(error)
  }
}
