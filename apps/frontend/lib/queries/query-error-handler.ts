/**
 * Utility functions for handling TanStack Query errors consistently.
 */

import { BackendApiError } from '@/lib/errors'
import type {
  ActionResult,
  SimpleActionResult,
  SnapshotActionResult,
} from '@/lib/api/action-helpers'

type ActionErrorResult =
  | Extract<ActionResult<unknown>, { kind: 'error' }>
  | Extract<SimpleActionResult, { kind: 'error' }>
  | Extract<SnapshotActionResult<unknown>, { kind: 'error' }>

/**
 * Convert an ActionResult error to a BackendApiError.
 * This is the primary utility for converting server action errors to throwable errors.
 */
export function handleActionResultError(
  errorResult: ActionErrorResult
): BackendApiError {
  return new BackendApiError(
    errorResult.message,
    errorResult.status,
    errorResult.code,
    errorResult.traceId
  )
}

/**
 * Convert a TanStack Query error to a BackendApiError.
 * Handles both BackendApiError instances and generic errors.
 * Use this when catching errors from query/mutation functions.
 * @param error - The error to convert
 * @param defaultMessage - Custom default message if error is not an Error instance (default: 'An unexpected error occurred')
 */
export function toQueryError(
  error: unknown,
  defaultMessage: string = 'An unexpected error occurred'
): BackendApiError {
  if (error instanceof BackendApiError) {
    return error
  }

  return new BackendApiError(
    error instanceof Error ? error.message : defaultMessage,
    500,
    'UNKNOWN_ERROR'
  )
}

/**
 * Convert a query error to a GameRoomError for display in game room components.
 * Handles both Error instances and unknown error types.
 */
export function getGameRoomError(error: unknown): { message: string } | null {
  if (!error) {
    return null
  }
  // Inline error message extraction (previously getErrorMessage)
  let message: string
  if (error instanceof BackendApiError) {
    message = error.message
  } else if (error instanceof Error) {
    message = error.message
  } else {
    message = 'An unexpected error occurred'
  }
  return { message }
}
