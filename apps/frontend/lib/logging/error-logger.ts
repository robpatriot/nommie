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
    errorDetails.error = error
  }

  if (context) {
    errorDetails.context = context
  }

  // In development, log with full details
  if (process.env.NODE_ENV === 'development') {
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
  logError(message, error, {
    traceId: error.traceId,
    status: error.status,
    code: error.code,
    ...additionalContext,
  })
}
