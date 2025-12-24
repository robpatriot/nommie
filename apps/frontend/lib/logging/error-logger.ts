/**
 * Centralized error logging service.
 * Provides consistent error logging across the application with support for
 * different log levels and optional error context.
 */

type ErrorContext = {
  traceId?: string
  gameId?: number
  userId?: string
  action?: string
  [key: string]: unknown
}

// LogLevel type reserved for future use (e.g., filtering by level)
// type LogLevel = 'error' | 'warn' | 'info'

/**
 * Extract serializable properties from an error object.
 * Error objects don't serialize their custom properties when logged,
 * so we extract them into a plain object.
 */
function serializeError(error: unknown): Record<string, unknown> | unknown {
  if (error instanceof Error) {
    const serialized: Record<string, unknown> = {
      name: error.name,
      message: error.message,
    }

    // Include stack trace if available
    if (error.stack) {
      serialized.stack = error.stack
    }

    // Extract custom properties from Error instances (e.g., BackendApiError)
    if ('status' in error) {
      serialized.status = (error as { status?: unknown }).status
    }
    if ('code' in error) {
      serialized.code = (error as { code?: unknown }).code
    }
    if ('traceId' in error) {
      serialized.traceId = (error as { traceId?: unknown }).traceId
    }

    return serialized
  }

  // If it's a plain object, return it as-is
  if (error && typeof error === 'object') {
    return error
  }

  return error
}

/**
 * Log an error with optional context.
 * In development, logs to console with full details.
 * In production, can be extended to send to error tracking service.
 */
export function logError(
  message: string,
  error?: unknown,
  context?: ErrorContext
): void {
  const errorDetails: {
    message: string
    error?: unknown
    context?: ErrorContext
    timestamp: string
  } = {
    message,
    timestamp: new Date().toISOString(),
  }

  if (error) {
    errorDetails.error = serializeError(error)
  }

  if (context) {
    errorDetails.context = context
  }

  // In development, log with full details
  if (process.env.NODE_ENV === 'development') {
    // Log the serialized error details (error has already been serialized)
    console.error('[Error Logger]', errorDetails)

    // Also log traceId if available for easier debugging
    if (context?.traceId) {
      console.error(`[Error Logger] Trace ID: ${context.traceId}`)
    }
  } else {
    // In production, log minimal details (can be extended to send to error tracking service)
    console.error(`[Error] ${message}`, error || '')
  }
}

/**
 * Log a warning with optional context.
 */
export function logWarning(message: string, context?: ErrorContext): void {
  if (process.env.NODE_ENV === 'development') {
    console.warn('[Warning Logger]', {
      message,
      context,
      timestamp: new Date().toISOString(),
    })
  } else {
    console.warn(`[Warning] ${message}`)
  }
}

/**
 * Log an informational message (for debugging).
 * Only logs in development.
 */
export function logInfo(message: string, context?: ErrorContext): void {
  if (process.env.NODE_ENV === 'development') {
    console.info('[Info Logger]', {
      message,
      context,
      timestamp: new Date().toISOString(),
    })
  }
}

/**
 * Log a BackendApiError with traceId for easier debugging.
 */
export function logBackendError(
  message: string,
  error: { message: string; traceId?: string; status?: number; code?: string },
  additionalContext?: Omit<ErrorContext, 'traceId'>
): void {
  // Extract error properties into a plain object for consistent serialization
  // The error parameter may be a BackendApiError instance or a plain object
  const errorDetails: Record<string, unknown> = {
    message: error.message,
  }
  if (error.traceId !== undefined) {
    errorDetails.traceId = error.traceId
  }
  if (error.status !== undefined) {
    errorDetails.status = error.status
  }
  if (error.code !== undefined) {
    errorDetails.code = error.code
  }

  // If it's an Error instance, include standard Error properties
  if (error instanceof Error) {
    errorDetails.name = error.name
    if (error.stack) {
      errorDetails.stack = error.stack
    }
  }

  logError(message, errorDetails, {
    traceId: error.traceId,
    status: error.status,
    code: error.code,
    ...additionalContext,
  })
}
