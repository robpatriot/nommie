// Server-only utility functions for converting errors to action result formats

import { BackendApiError } from '@/lib/errors'

/**
 * Generic action result type with discriminated union.
 * Use this for actions that return data.
 */
export type ActionResult<T = void> =
  | { kind: 'ok'; data: T }
  | {
      kind: 'error'
      message: string
      status: number
      code?: string
      traceId?: string
    }

/**
 * Simple action result type for actions that return void.
 * Use this for actions that only succeed or fail.
 */
export type SimpleActionResult =
  | { kind: 'ok' }
  | {
      kind: 'error'
      message: string
      status: number
      code?: string
      traceId?: string
    }

/**
 * Snapshot action result type for actions that support ETag/conditional requests.
 * Extends ActionResult with a 'not_modified' variant for 304 responses.
 * Use this for actions that fetch snapshots and need to handle cache hits.
 */
export type SnapshotActionResult<T> = ActionResult<T> | { kind: 'not_modified' }

/**
 * Convert an error (BackendApiError or unknown) to an error result.
 * Wraps unexpected errors in BackendApiError for consistent error handling.
 */
export function toErrorResult(
  error: unknown,
  defaultMessage: string,
  defaultStatus: number = 500
): Extract<SimpleActionResult, { kind: 'error' }> {
  if (error instanceof BackendApiError) {
    return {
      kind: 'error',
      message: error.message,
      status: error.status,
      code: error.code,
      traceId: error.traceId,
    }
  }

  // Wrap unexpected errors in BackendApiError for consistent error handling
  const wrappedError = new BackendApiError(
    error instanceof Error ? error.message : defaultMessage,
    defaultStatus,
    'UNKNOWN_ERROR'
  )
  return {
    kind: 'error',
    message: wrappedError.message,
    status: wrappedError.status,
    code: wrappedError.code,
    traceId: wrappedError.traceId,
  }
}
