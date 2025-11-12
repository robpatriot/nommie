// Server-only utility for parsing Problem Details error responses (RFC 7807)

export interface ProblemDetails {
  type: string
  title: string
  status: number
  detail: string
  code: string
  trace_id: string
  extensions?: unknown
}

export interface ParsedError {
  message: string
  code?: string
  traceId?: string
}

/**
 * Parse Problem Details error response from a Response object.
 * Returns the error message, code, and traceId if available.
 */
export async function parseErrorResponse(
  response: Response
): Promise<ParsedError> {
  let errorMessage = response.statusText
  let errorCode: string | undefined
  let traceId = response.headers.get('x-trace-id') || undefined

  // Try to parse Problem Details error response (RFC 7807)
  const contentType = response.headers.get('content-type')
  const isProblemDetails =
    contentType?.includes('application/problem+json') ||
    contentType?.includes('application/json')

  if (isProblemDetails) {
    try {
      const problemDetails: ProblemDetails = await response.clone().json()
      errorMessage =
        problemDetails.detail || problemDetails.title || errorMessage
      errorCode = problemDetails.code
      traceId = problemDetails.trace_id || traceId
    } catch {
      // If parsing fails, fall back to status text
    }
  }

  // For 401, ensure we have a proper error code
  if (response.status === 401) {
    errorCode = errorCode || 'UNAUTHORIZED'
    errorMessage = errorMessage || 'Unauthorized'
  }

  return {
    message: errorMessage,
    code: errorCode,
    traceId,
  }
}
