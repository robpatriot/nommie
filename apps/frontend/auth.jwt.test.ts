import { beforeEach, describe, expect, it, vi } from 'vitest'

const mockLogError = vi.fn()
vi.mock('@/lib/logging/error-logger', () => ({
  logError: (...args: unknown[]) => mockLogError(...args),
}))

describe('Google sign-in id_token validation', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('fails fast when account.id_token is missing: logs error and throws (no fallback to profile)', async () => {
    const { requireGoogleIdToken } =
      await import('@/lib/auth/require-google-id-token')

    const account = { provider: 'google' } as {
      provider: string
      id_token?: string
    }

    await expect(requireGoogleIdToken(account)).rejects.toThrow(
      'Google sign-in failed: missing id_token'
    )

    expect(mockLogError).toHaveBeenCalledWith(
      'Google id_token missing at sign-in',
      expect.any(Error),
      { action: 'initialLogin' }
    )
  })

  it('fails when id_token is empty string', async () => {
    const { requireGoogleIdToken } =
      await import('@/lib/auth/require-google-id-token')

    const account = { provider: 'google', id_token: '' }

    await expect(requireGoogleIdToken(account)).rejects.toThrow(
      'Google sign-in failed: missing id_token'
    )
    expect(mockLogError).toHaveBeenCalled()
  })

  it('returns id_token when valid', async () => {
    const { requireGoogleIdToken } =
      await import('@/lib/auth/require-google-id-token')

    const account = { provider: 'google', id_token: 'valid-jwt-token' }

    const result = await requireGoogleIdToken(account)
    expect(result).toBe('valid-jwt-token')
    expect(mockLogError).not.toHaveBeenCalled()
  })
})
