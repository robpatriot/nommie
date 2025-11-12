// Server-only utility functions for converting errors to action result formats

import { BackendApiError } from '@/lib/errors'

/**
 * Error result format for actions that return SimpleActionResult
 */
export interface ErrorResult {
  kind: 'error'
  message: string
  status: number
  code?: string
  traceId?: string
}

/**
 * Convert an error (BackendApiError or unknown) to an ErrorResult.
 * Wraps unexpected errors in BackendApiError for consistent error handling.
 */
export function toErrorResult(
  error: unknown,
  defaultMessage: string,
  defaultStatus: number = 500
): ErrorResult {
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

/**
 * Convert a BackendApiError to an ErrorResult.
 * Use this when you know the error is already a BackendApiError.
 */
export function backendErrorToResult(error: BackendApiError): ErrorResult {
  return {
    kind: 'error',
    message: error.message,
    status: error.status,
    code: error.code,
    traceId: error.traceId,
  }
}
