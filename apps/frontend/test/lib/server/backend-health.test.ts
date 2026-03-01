import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { probeBackendReadiness } from '@/lib/server/backend-health'

describe('probeBackendReadiness', () => {
  const origBackend = process.env.BACKEND_BASE_URL
  const origPublic = process.env.NEXT_PUBLIC_BACKEND_BASE_URL

  beforeEach(() => {
    delete process.env.BACKEND_BASE_URL
    delete process.env.NEXT_PUBLIC_BACKEND_BASE_URL
    vi.spyOn(global, 'fetch').mockImplementation(() =>
      Promise.resolve({ ok: true } as Response)
    )
  })

  afterEach(() => {
    if (origBackend !== undefined) process.env.BACKEND_BASE_URL = origBackend
    if (origPublic !== undefined)
      process.env.NEXT_PUBLIC_BACKEND_BASE_URL = origPublic
    vi.restoreAllMocks()
  })

  it('returns ready:false without throwing when no env and no sameOriginFallback', async () => {
    await expect(probeBackendReadiness()).resolves.toEqual({
      ready: false,
      error: 'Backend URL not configured',
    })
    expect(global.fetch).not.toHaveBeenCalled()
  })

  it('returns ready:false without throwing when sameOriginFallback is empty string', async () => {
    await expect(probeBackendReadiness('')).resolves.toEqual({
      ready: false,
      error: 'Backend URL not configured',
    })
    expect(global.fetch).not.toHaveBeenCalled()
  })

  it('uses sameOriginFallback when env is unset and fetches /api/readyz', async () => {
    await expect(
      probeBackendReadiness('https://app.example.com')
    ).resolves.toEqual({ ready: true })
    expect(global.fetch).toHaveBeenCalledWith(
      'https://app.example.com/api/readyz',
      expect.objectContaining({
        method: 'GET',
        signal: expect.any(AbortSignal),
      })
    )
  })
})
