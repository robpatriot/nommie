import React from 'react'
import { render, act } from '@testing-library/react'
import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import {
  BackendReadinessProvider,
  useBackendReadiness,
} from '@/lib/providers/backend-readiness-provider'
import { ManualPollDriver } from '@/test/utils'

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
    vi.stubEnv('NEXT_PUBLIC_FETCH_MODE', 'test')
    vi.spyOn(global, 'fetch')

    if (typeof AbortSignal.timeout !== 'function') {
      ;(AbortSignal as any).timeout = () => new AbortController().signal
    }
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('starts in ready state and does not poll on mount', () => {
    const fetchSpy = vi.mocked(global.fetch)

    const { getByTestId } = render(
      <BackendReadinessProvider>
        <TestConsumer />
      </BackendReadinessProvider>
    )

    expect(getByTestId('readiness').textContent).toBe('ready')
    expect(fetchSpy).not.toHaveBeenCalled()
  })

  it('shows not-ready after triggerRecovery and polls frontend /readyz', async () => {
    const driver = new ManualPollDriver()
    const fetchSpy = vi.mocked(global.fetch)
    fetchSpy.mockResolvedValueOnce({ ok: true } as Response)

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

    expect(getByTestId('readiness').textContent).toBe('ready')

    act(() => {
      triggerRecovery()
    })
    expect(getByTestId('readiness').textContent).toBe('not-ready')

    await act(async () => {
      await driver.tick()
    })
    expect(getByTestId('readiness').textContent).toBe('ready')
    expect(fetchSpy).toHaveBeenCalledTimes(1)
    expect(fetchSpy).toHaveBeenCalledWith(
      '/readyz',
      expect.objectContaining({ method: 'GET' })
    )
  })

  it('exits degraded on first 200 and stops polling', async () => {
    const driver = new ManualPollDriver()
    const fetchSpy = vi.mocked(global.fetch)
    fetchSpy.mockResolvedValue({ ok: true } as Response)

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

    act(() => {
      triggerRecovery()
    })
    expect(getByTestId('readiness').textContent).toBe('not-ready')

    await act(async () => {
      await driver.tick()
    })
    expect(getByTestId('readiness').textContent).toBe('ready')

    expect(fetchSpy).toHaveBeenCalledTimes(1)
  })

  it('stays not-ready while probe returns non-200 and recovers on first 200', async () => {
    const driver = new ManualPollDriver()
    const fetchSpy = vi.mocked(global.fetch)
    fetchSpy
      .mockResolvedValueOnce({ ok: false } as Response)
      .mockResolvedValueOnce({ ok: false } as Response)
      .mockResolvedValueOnce({ ok: true } as Response)

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

    act(() => {
      triggerRecovery()
    })
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
})
