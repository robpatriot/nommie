import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { parseErrorResponse, type ProblemDetails } from './error-parsing'

describe('parseErrorResponse', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('parses Problem Details response with detail message', async () => {
    const problemDetails: ProblemDetails = {
      type: 'https://example.com/problems/validation-error',
      title: 'Validation Error',
      status: 400,
      detail: 'Invalid bid value: must be between 0 and 13',
      code: 'VALIDATION_ERROR',
      trace_id: 'trace-123',
    }

    const response = new Response(JSON.stringify(problemDetails), {
      status: 400,
      statusText: 'Bad Request',
      headers: {
        'Content-Type': 'application/problem+json',
        'x-trace-id': 'header-trace-id',
      },
    })

    const result = await parseErrorResponse(response)

    expect(result.message).toBe('Invalid bid value: must be between 0 and 13')
    expect(result.code).toBe('VALIDATION_ERROR')
    expect(result.traceId).toBe('trace-123') // Body trace_id takes precedence
  })

  it('parses Problem Details response with title fallback', async () => {
    const problemDetails: ProblemDetails = {
      type: 'https://example.com/problems/server-error',
      title: 'Internal Server Error',
      status: 500,
      detail: '',
      code: 'SERVER_ERROR',
      trace_id: 'trace-456',
    }

    const response = new Response(JSON.stringify(problemDetails), {
      status: 500,
      statusText: 'Internal Server Error',
      headers: {
        'Content-Type': 'application/problem+json',
      },
    })

    const result = await parseErrorResponse(response)

    expect(result.message).toBe('Internal Server Error') // Falls back to title when detail is empty
    expect(result.code).toBe('SERVER_ERROR')
    expect(result.traceId).toBe('trace-456')
  })

  it('falls back to status text when Problem Details parsing fails', async () => {
    const response = new Response('Invalid JSON', {
      status: 500,
      statusText: 'Internal Server Error',
      headers: {
        'Content-Type': 'application/problem+json',
      },
    })

    const result = await parseErrorResponse(response)

    expect(result.message).toBe('Internal Server Error')
    expect(result.code).toBeUndefined()
    expect(result.traceId).toBeUndefined()
  })

  it('uses status text for non-JSON responses', async () => {
    const response = new Response('Not Found', {
      status: 404,
      statusText: 'Not Found',
      headers: {
        'Content-Type': 'text/plain',
      },
    })

    const result = await parseErrorResponse(response)

    expect(result.message).toBe('Not Found')
    expect(result.code).toBeUndefined()
    expect(result.traceId).toBeUndefined()
  })

  it('extracts trace ID from header when not in body', async () => {
    const response = new Response('Error occurred', {
      status: 500,
      statusText: 'Internal Server Error',
      headers: {
        'Content-Type': 'application/json',
        'x-trace-id': 'header-trace-id',
      },
    })

    // Mock JSON parsing to return empty object (no trace_id in body)
    const mockClone = vi.spyOn(Response.prototype, 'clone')
    const mockResponse = {
      json: vi.fn().mockResolvedValue({}),
    } as unknown as Response
    mockClone.mockReturnValue(mockResponse)

    const result = await parseErrorResponse(response)

    expect(result.traceId).toBe('header-trace-id')
  })

  it('handles 401 errors with proper error code', async () => {
    const response = new Response('Unauthorized', {
      status: 401,
      statusText: 'Unauthorized',
      headers: {
        'Content-Type': 'text/plain',
      },
    })

    const result = await parseErrorResponse(response)

    expect(result.message).toBe('Unauthorized')
    expect(result.code).toBe('UNAUTHORIZED')
  })

  it('preserves error code from Problem Details for 401', async () => {
    const problemDetails: ProblemDetails = {
      type: 'https://example.com/problems/unauthorized',
      title: 'Unauthorized',
      status: 401,
      detail: 'Token expired',
      code: 'TOKEN_EXPIRED',
      trace_id: 'trace-789',
    }

    const response = new Response(JSON.stringify(problemDetails), {
      status: 401,
      statusText: 'Unauthorized',
      headers: {
        'Content-Type': 'application/problem+json',
      },
    })

    const result = await parseErrorResponse(response)

    expect(result.message).toBe('Token expired')
    expect(result.code).toBe('TOKEN_EXPIRED') // Preserves custom code
    expect(result.traceId).toBe('trace-789')
  })

  it('handles JSON content type with Problem Details', async () => {
    const problemDetails: ProblemDetails = {
      type: 'https://example.com/problems/bad-request',
      title: 'Bad Request',
      status: 400,
      detail: 'Invalid request body',
      code: 'BAD_REQUEST',
      trace_id: 'trace-abc',
    }

    const response = new Response(JSON.stringify(problemDetails), {
      status: 400,
      statusText: 'Bad Request',
      headers: {
        'Content-Type': 'application/json',
      },
    })

    const result = await parseErrorResponse(response)

    expect(result.message).toBe('Invalid request body')
    expect(result.code).toBe('BAD_REQUEST')
    expect(result.traceId).toBe('trace-abc')
  })

  it('handles empty response body gracefully', async () => {
    const response = new Response('', {
      status: 500,
      statusText: 'Internal Server Error',
      headers: {
        'Content-Type': 'application/json',
      },
    })

    const result = await parseErrorResponse(response)

    expect(result.message).toBe('Internal Server Error')
    expect(result.code).toBeUndefined()
  })

  it('handles response without content-type header', async () => {
    const response = new Response('Error', {
      status: 500,
      statusText: 'Internal Server Error',
      headers: {},
    })

    const result = await parseErrorResponse(response)

    expect(result.message).toBe('Internal Server Error')
    expect(result.code).toBeUndefined()
  })
})
