import { describe, it, expect } from 'vitest'
import { BackendApiError } from '@/lib/errors'
import { toErrorResult, backendErrorToResult } from './action-helpers'

describe('toErrorResult', () => {
  it('converts BackendApiError to ErrorResult', () => {
    const error = new BackendApiError(
      'Invalid bid value',
      400,
      'VALIDATION_ERROR',
      'trace-123'
    )

    const result = toErrorResult(error, 'Failed to submit bid')

    expect(result).toEqual({
      kind: 'error',
      message: 'Invalid bid value',
      status: 400,
      code: 'VALIDATION_ERROR',
      traceId: 'trace-123',
    })
  })

  it('converts Error to ErrorResult with default message', () => {
    const error = new Error('Network timeout')

    const result = toErrorResult(error, 'Failed to submit bid')

    expect(result).toEqual({
      kind: 'error',
      message: 'Network timeout',
      status: 500,
      code: 'UNKNOWN_ERROR',
      traceId: undefined,
    })
  })

  it('converts Error to ErrorResult with custom default status', () => {
    const error = new Error('Validation failed')

    const result = toErrorResult(error, 'Failed to validate', 400)

    expect(result).toEqual({
      kind: 'error',
      message: 'Validation failed',
      status: 400,
      code: 'UNKNOWN_ERROR',
      traceId: undefined,
    })
  })

  it('converts unknown error type to ErrorResult', () => {
    const error = { customProperty: 'value' }

    const result = toErrorResult(error, 'Failed to submit bid')

    expect(result).toEqual({
      kind: 'error',
      message: 'Failed to submit bid',
      status: 500,
      code: 'UNKNOWN_ERROR',
      traceId: undefined,
    })
  })

  it('converts null to ErrorResult', () => {
    const result = toErrorResult(null, 'Failed to submit bid')

    expect(result).toEqual({
      kind: 'error',
      message: 'Failed to submit bid',
      status: 500,
      code: 'UNKNOWN_ERROR',
      traceId: undefined,
    })
  })

  it('converts undefined to ErrorResult', () => {
    const result = toErrorResult(undefined, 'Failed to submit bid')

    expect(result).toEqual({
      kind: 'error',
      message: 'Failed to submit bid',
      status: 500,
      code: 'UNKNOWN_ERROR',
      traceId: undefined,
    })
  })

  it('preserves BackendApiError traceId', () => {
    const error = new BackendApiError(
      'Server error',
      500,
      'SERVER_ERROR',
      'trace-abc-123'
    )

    const result = toErrorResult(error, 'Failed to submit bid')

    expect(result.traceId).toBe('trace-abc-123')
  })

  it('handles BackendApiError without optional fields', () => {
    const error = new BackendApiError('Error message', 500)

    const result = toErrorResult(error, 'Failed to submit bid')

    expect(result).toEqual({
      kind: 'error',
      message: 'Error message',
      status: 500,
      code: undefined,
      traceId: undefined,
    })
  })
})

describe('backendErrorToResult', () => {
  it('converts BackendApiError to ErrorResult', () => {
    const error = new BackendApiError(
      'Invalid bid value',
      400,
      'VALIDATION_ERROR',
      'trace-123'
    )

    const result = backendErrorToResult(error)

    expect(result).toEqual({
      kind: 'error',
      message: 'Invalid bid value',
      status: 400,
      code: 'VALIDATION_ERROR',
      traceId: 'trace-123',
    })
  })

  it('handles BackendApiError without optional fields', () => {
    const error = new BackendApiError('Error message', 500)

    const result = backendErrorToResult(error)

    expect(result).toEqual({
      kind: 'error',
      message: 'Error message',
      status: 500,
      code: undefined,
      traceId: undefined,
    })
  })

  it('preserves all error properties', () => {
    const error = new BackendApiError(
      'Custom error message',
      404,
      'NOT_FOUND',
      'trace-xyz'
    )

    const result = backendErrorToResult(error)

    expect(result.message).toBe('Custom error message')
    expect(result.status).toBe(404)
    expect(result.code).toBe('NOT_FOUND')
    expect(result.traceId).toBe('trace-xyz')
  })
})
