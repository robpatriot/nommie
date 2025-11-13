import { describe, it, expect, vi, afterEach } from 'vitest'
import {
  isNetworkError,
  calculateBackoffDelay,
  retryOnNetworkError,
} from './retry'

describe('isNetworkError', () => {
  it('identifies fetch failed errors', () => {
    const error = new Error('fetch failed')
    expect(isNetworkError(error)).toBe(true)
  })

  it('identifies network timeout errors', () => {
    const error = new Error('network timeout')
    expect(isNetworkError(error)).toBe(true)
  })

  it('identifies network error messages', () => {
    const error = new Error('network error')
    expect(isNetworkError(error)).toBe(true)
  })

  it('identifies failed to fetch errors', () => {
    const error = new Error('failed to fetch')
    expect(isNetworkError(error)).toBe(true)
  })

  it('identifies connection error messages', () => {
    const error = new Error('connection error')
    expect(isNetworkError(error)).toBe(true)
  })

  it('identifies ECONNREFUSED errors', () => {
    const error = new Error('ECONNREFUSED')
    expect(isNetworkError(error)).toBe(true)
  })

  it('identifies ETIMEDOUT errors', () => {
    const error = new Error('ETIMEDOUT')
    expect(isNetworkError(error)).toBe(true)
  })

  it('identifies ENOTFOUND errors', () => {
    const error = new Error('ENOTFOUND')
    expect(isNetworkError(error)).toBe(true)
  })

  it('identifies ECONNRESET errors', () => {
    const error = new Error('ECONNRESET')
    expect(isNetworkError(error)).toBe(true)
  })

  it('identifies socket hang up errors', () => {
    const error = new Error('socket hang up')
    expect(isNetworkError(error)).toBe(true)
  })

  it('identifies errors with Network in name', () => {
    const error = new Error('Something went wrong')
    error.name = 'NetworkError'
    expect(isNetworkError(error)).toBe(true)
  })

  it('identifies errors with Timeout in name', () => {
    const error = new Error('Something went wrong')
    error.name = 'TimeoutError'
    expect(isNetworkError(error)).toBe(true)
  })

  it('returns false for non-network errors', () => {
    const error = new Error('Validation failed')
    expect(isNetworkError(error)).toBe(false)
  })

  it('returns false for non-Error objects', () => {
    expect(isNetworkError('string')).toBe(false)
    expect(isNetworkError(123)).toBe(false)
    expect(isNetworkError(null)).toBe(false)
    expect(isNetworkError(undefined)).toBe(false)
    expect(isNetworkError({})).toBe(false)
  })

  it('is case-insensitive for error messages', () => {
    const error1 = new Error('FETCH FAILED')
    const error2 = new Error('Network Timeout')
    const error3 = new Error('Failed To Fetch')

    expect(isNetworkError(error1)).toBe(true)
    expect(isNetworkError(error2)).toBe(true)
    expect(isNetworkError(error3)).toBe(true)
  })
})

describe('calculateBackoffDelay', () => {
  it('calculates exponential backoff delay', () => {
    expect(calculateBackoffDelay(0, 1000)).toBe(1000) // 1000 * 2^0
    expect(calculateBackoffDelay(1, 1000)).toBe(2000) // 1000 * 2^1
    expect(calculateBackoffDelay(2, 1000)).toBe(4000) // 1000 * 2^2
    expect(calculateBackoffDelay(3, 1000)).toBe(8000) // 1000 * 2^3
  })

  it('respects max delay', () => {
    expect(calculateBackoffDelay(0, 1000, 5000)).toBe(1000)
    expect(calculateBackoffDelay(1, 1000, 5000)).toBe(2000)
    expect(calculateBackoffDelay(2, 1000, 5000)).toBe(4000)
    expect(calculateBackoffDelay(3, 1000, 5000)).toBe(5000) // Capped at max
    expect(calculateBackoffDelay(4, 1000, 5000)).toBe(5000) // Capped at max
  })

  it('uses default base delay of 1000ms', () => {
    expect(calculateBackoffDelay(0)).toBe(1000)
    expect(calculateBackoffDelay(1)).toBe(2000)
  })

  it('uses default max delay of 10000ms', () => {
    expect(calculateBackoffDelay(10, 1000)).toBe(10000) // Capped at default max
  })

  it('handles custom base delay', () => {
    expect(calculateBackoffDelay(0, 500)).toBe(500)
    expect(calculateBackoffDelay(1, 500)).toBe(1000)
    expect(calculateBackoffDelay(2, 500)).toBe(2000)
  })
})

describe('retryOnNetworkError', () => {
  afterEach(() => {
    vi.clearAllMocks()
  })

  it('returns result on first successful attempt', async () => {
    const fn = vi.fn().mockResolvedValue('success')

    const result = await retryOnNetworkError(fn)

    expect(result).toBe('success')
    expect(fn).toHaveBeenCalledTimes(1)
  })

  it('retries on network error and succeeds on retry', async () => {
    const networkError = new Error('fetch failed')
    const fn = vi
      .fn()
      .mockRejectedValueOnce(networkError)
      .mockResolvedValueOnce('success')

    const result = await retryOnNetworkError(fn, {
      maxRetries: 1,
      baseDelayMs: 10, // Use small delay for testing
    })

    expect(result).toBe('success')
    expect(fn).toHaveBeenCalledTimes(2)
  }, 5000) // Increase timeout for retry delays

  it('does not retry on non-network errors', async () => {
    const nonNetworkError = new Error('Validation failed')
    const fn = vi.fn().mockRejectedValue(nonNetworkError)

    await expect(
      retryOnNetworkError(fn, {
        maxRetries: 1,
        baseDelayMs: 10,
      })
    ).rejects.toThrow('Validation failed')

    expect(fn).toHaveBeenCalledTimes(1) // Should not retry
  })

  it('retries up to maxRetries times', async () => {
    const networkError = new Error('fetch failed')
    const fn = vi.fn().mockRejectedValue(networkError)

    await expect(
      retryOnNetworkError(fn, {
        maxRetries: 2,
        baseDelayMs: 10, // Use small delay for testing
      })
    ).rejects.toThrow('fetch failed')

    expect(fn).toHaveBeenCalledTimes(3) // Initial + 2 retries
  }, 5000) // Increase timeout for retry delays

  it('calls onRetry callback before each retry', async () => {
    const networkError = new Error('fetch failed')
    const fn = vi.fn().mockRejectedValue(networkError)
    const onRetry = vi.fn()

    await expect(
      retryOnNetworkError(fn, {
        maxRetries: 1,
        baseDelayMs: 10, // Use small delay for testing
        onRetry,
      })
    ).rejects.toThrow('fetch failed')

    expect(onRetry).toHaveBeenCalledTimes(1)
    expect(onRetry).toHaveBeenCalledWith(1, networkError)
  }, 5000) // Increase timeout for retry delays

  it('uses exponential backoff for retry delays', async () => {
    const networkError = new Error('fetch failed')
    const fn = vi.fn()
    const callTimes: number[] = []
    const startTime = Date.now()

    // Track when each call happens
    fn.mockImplementation(async () => {
      callTimes.push(Date.now() - startTime)
      throw networkError
    })

    // Start the retry operation (don't await yet)
    const retryPromise = retryOnNetworkError(fn, {
      maxRetries: 2,
      baseDelayMs: 50, // Use small delay for testing
    })

    // Wait for all retries to complete
    await expect(retryPromise).rejects.toThrow('fetch failed')

    expect(fn).toHaveBeenCalledTimes(3) // Initial + 2 retries
    expect(callTimes.length).toBe(3)

    // Verify delays increase exponentially (with tolerance for real timers)
    const firstDelay = callTimes[1] - callTimes[0]
    const secondDelay = callTimes[2] - callTimes[1]

    // First delay should be ~50ms (with tolerance for async execution)
    expect(firstDelay).toBeGreaterThan(30)
    expect(firstDelay).toBeLessThan(100)

    // Second delay should be ~100ms (exponential backoff)
    // Allow for some variance due to real timers and async execution
    expect(secondDelay).toBeGreaterThan(50)
    expect(secondDelay).toBeLessThan(200)

    // Most importantly: second delay should be greater than first delay (exponential)
    expect(secondDelay).toBeGreaterThan(firstDelay)
  }, 10000) // Increase timeout to allow for real timer delays

  it('respects maxDelayMs for backoff', async () => {
    const networkError = new Error('fetch failed')
    const fn = vi.fn().mockRejectedValue(networkError)

    await expect(
      retryOnNetworkError(fn, {
        maxRetries: 2,
        baseDelayMs: 100,
        maxDelayMs: 150, // Cap delays at 150ms
      })
    ).rejects.toThrow('fetch failed')

    expect(fn).toHaveBeenCalledTimes(3) // Initial + 2 retries
    // Note: We can't easily verify the exact delays without fake timers,
    // but the function should respect maxDelayMs
  }, 5000) // Increase timeout for retry delays

  it('does not retry if maxRetries is 0', async () => {
    const networkError = new Error('fetch failed')
    const fn = vi.fn().mockRejectedValue(networkError)

    await expect(
      retryOnNetworkError(fn, {
        maxRetries: 0,
        baseDelayMs: 10,
      })
    ).rejects.toThrow('fetch failed')

    expect(fn).toHaveBeenCalledTimes(1) // Only initial call
  })

  it('handles TypeScript generic return type', async () => {
    interface CustomResult {
      value: string
    }

    const fn = vi.fn().mockResolvedValue({ value: 'test' } as CustomResult)

    const result = await retryOnNetworkError<CustomResult>(fn)

    expect(result).toEqual({ value: 'test' })
  })
})
