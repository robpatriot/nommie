// Client-safe error classes (can be imported from client components)

export class BackendApiError extends Error {
  constructor(
    message: string,
    public status: number,
    public code?: string,
    public traceId?: string
  ) {
    super(message)
    this.name = 'BackendApiError'
  }
}

/**
 * Check if an error represents a stale user session.
 * Stale sessions occur when:
 * - 401: User is not authorized (generic)
 * - 401 MISSING_BACKEND_JWT: No JWT cookie available
 * - 401 FORBIDDEN_USER_NOT_FOUND: JWT valid but user no longer in DB (e.g. DB wipe)
 */
export function isStaleSessionError(error: unknown): boolean {
  if (!(error instanceof BackendApiError)) {
    return false
  }

  // Treat all 401 errors as stale sessions to trigger a clean redirect
  if (error.status === 401) {
    return true
  }

  // Match specific codes if they're not 401 (though usually they are)
  const staleCodes = ['FORBIDDEN_USER_NOT_FOUND', 'MISSING_BACKEND_JWT']
  if (error.code && staleCodes.includes(error.code)) {
    return true
  }

  return false
}
