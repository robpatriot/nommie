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
  // Runtime validation - defensive programming
  // TypeScript ensures this at compile time, but runtime check prevents issues
  // if the function is called incorrectly (e.g., type assertion bugs, JS interop)
  if (errorResult.kind !== 'error') {
    throw new Error(
      `handleActionResultError called with non-error result. Expected kind: 'error', got: '${errorResult.kind}'`
    )
  }

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
 * Only surfaces structured BackendApiError instances (which are already localized
 * via error codes); other unexpected errors are treated as null here and are
 * handled by higher-level toasts or loggers.
 */
export function getGameRoomError(
  error: unknown
): { message: string; traceId?: string } | null {
  if (!error) {
    return null
  }
  if (error instanceof BackendApiError) {
    return { message: error.message, traceId: error.traceId }
  }
  return null
}
