import React from 'react'
import { render, act } from '@testing-library/react'
import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import {
  BackendReadinessProvider,
  useBackendReadiness,
} from '@/lib/providers/backend-readiness-provider'
import { ManualPollDriver } from '@/test/utils'

// Helper component to observe context values
function TestConsumer({
  onState,
}: {
  onState?: (val: { isReady: boolean; triggerRecovery: () => void }) => void
}) {
  const state = useBackendReadiness()
  onState?.(state)
  return (
    <div data-testid="readiness">{state.isReady ? 'ready' : 'not-ready'}</div>
  )
}

describe('BackendReadinessProvider', () => {
  beforeEach(() => {
    vi.stubEnv('NEXT_PUBLIC_BACKEND_BASE_URL', 'http://localhost:3001')
    vi.stubEnv('NEXT_PUBLIC_FETCH_MODE', 'test')
    vi.spyOn(global, 'fetch')

    // If AbortSignal.timeout isn't available in the test runtime, shim it.
    if (typeof AbortSignal.timeout !== 'function') {
      ;(AbortSignal as any).timeout = () => new AbortController().signal
    }
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('starts in not-ready state and polls until ready', async () => {
    const driver = new ManualPollDriver()
    const fetchSpy = vi.mocked(global.fetch)

    // Poll 1: 503
    fetchSpy.mockResolvedValueOnce({ ok: false } as Response)
    // Poll 2: 200
    fetchSpy.mockResolvedValueOnce({ ok: true } as Response)
    // Poll 3: 200 (threshold -> ready)
    fetchSpy.mockResolvedValueOnce({ ok: true } as Response)

    const { getByTestId } = render(
      <BackendReadinessProvider pollDriver={driver}>
        <TestConsumer />
      </BackendReadinessProvider>
    )

    expect(getByTestId('readiness').textContent).toBe('not-ready')

    await act(async () => {
      await driver.tick()
    })
    expect(getByTestId('readiness').textContent).toBe('not-ready')

    await act(async () => {
      await driver.tick()
    })
    expect(getByTestId('readiness').textContent).toBe('not-ready')

    await act(async () => {
      await driver.tick()
    })
    expect(getByTestId('readiness').textContent).toBe('ready')

    expect(fetchSpy).toHaveBeenCalledTimes(3)
  })

  it('transitions to recovering and back to healthy', async () => {
    const driver = new ManualPollDriver()
    const fetchSpy = vi.mocked(global.fetch)

    let backendOk = true
    fetchSpy.mockImplementation(async () => ({ ok: backendOk }) as Response)

    let triggerRecovery: () => void = () => {}

    const { getByTestId } = render(
      <BackendReadinessProvider pollDriver={driver}>
        <TestConsumer
          onState={(s) => {
            triggerRecovery = s.triggerRecovery
          }}
        />
      </BackendReadinessProvider>
    )

    // Become healthy: needs 2 consecutive OKs
    await act(async () => {
      await driver.tick()
    }) // ok #1
    expect(getByTestId('readiness').textContent).toBe('not-ready')

    await act(async () => {
      await driver.tick()
    }) // ok #2 => ready
    expect(getByTestId('readiness').textContent).toBe('ready')

    // Trigger recovery
    backendOk = false
    act(() => {
      triggerRecovery()
    })
    expect(getByTestId('readiness').textContent).toBe('not-ready')

    // Poll at least once while backend failing
    await act(async () => {
      await driver.tick()
    })
    expect(getByTestId('readiness').textContent).toBe('not-ready')

    // Backend recovers: needs 2 consecutive OKs
    backendOk = true

    await act(async () => {
      await driver.tick()
    }) // ok #1
    expect(getByTestId('readiness').textContent).toBe('not-ready')

    await act(async () => {
      await driver.tick()
    }) // ok #2 => ready
    expect(getByTestId('readiness').textContent).toBe('ready')
  })
})
