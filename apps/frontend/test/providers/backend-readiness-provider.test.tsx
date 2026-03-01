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
  onState?: (val: ReturnType<typeof useBackendReadiness>) => void
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

  it('enters degraded immediately on reportFailure("permanent")', () => {
    let reportFailure: (kind: 'permanent' | 'transient') => void = () => {}

    const { getByTestId } = render(
      <BackendReadinessProvider>
        <TestConsumer
          onState={(s) => {
            reportFailure = s.reportFailure
          }}
        />
      </BackendReadinessProvider>
    )

    expect(getByTestId('readiness').textContent).toBe('ready')
    act(() => {
      reportFailure('permanent')
    })
    expect(getByTestId('readiness').textContent).toBe('not-ready')
  })

  it('enters degraded after 2 consecutive reportFailure("transient")', () => {
    let reportFailure: (kind: 'permanent' | 'transient') => void = () => {}

    const { getByTestId } = render(
      <BackendReadinessProvider>
        <TestConsumer
          onState={(s) => {
            reportFailure = s.reportFailure
          }}
        />
      </BackendReadinessProvider>
    )

    expect(getByTestId('readiness').textContent).toBe('ready')
    act(() => {
      reportFailure('transient')
    })
    expect(getByTestId('readiness').textContent).toBe('ready')
    act(() => {
      reportFailure('transient')
    })
    expect(getByTestId('readiness').textContent).toBe('not-ready')
  })

  it('resets transient count on reportSuccess()', () => {
    let reportFailure: (kind: 'permanent' | 'transient') => void = () => {}
    let reportSuccess: () => void = () => {}

    const { getByTestId } = render(
      <BackendReadinessProvider>
        <TestConsumer
          onState={(s) => {
            reportFailure = s.reportFailure
            reportSuccess = s.reportSuccess
          }}
        />
      </BackendReadinessProvider>
    )

    act(() => {
      reportFailure('transient')
    })
    expect(getByTestId('readiness').textContent).toBe('ready')
    act(() => {
      reportSuccess()
    })
    act(() => {
      reportFailure('transient')
    })
    expect(getByTestId('readiness').textContent).toBe('ready')
  })

  it('shows not-ready after triggerRecovery and exits after 2 consecutive successful /readyz probes', async () => {
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

    expect(getByTestId('readiness').textContent).toBe('ready')
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
    expect(getByTestId('readiness').textContent).toBe('ready')
    expect(fetchSpy).toHaveBeenCalledWith(
      '/readyz',
      expect.objectContaining({ method: 'GET' })
    )
  })

  it('stays not-ready while probe fails and exits after 2 consecutive successes', async () => {
    const driver = new ManualPollDriver()
    const fetchSpy = vi.mocked(global.fetch)
    fetchSpy
      .mockResolvedValueOnce({ ok: false } as Response)
      .mockResolvedValueOnce({ ok: false } as Response)
      .mockResolvedValueOnce({ ok: true } as Response)
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
    expect(getByTestId('readiness').textContent).toBe('not-ready')
    await act(async () => {
      await driver.tick()
    })
    expect(getByTestId('readiness').textContent).toBe('ready')
    expect(fetchSpy).toHaveBeenCalledTimes(4)
  })

  it('stops polling on unmount', async () => {
    const driver = new ManualPollDriver()
    const fetchSpy = vi.mocked(global.fetch)
    fetchSpy.mockResolvedValue({ ok: false } as Response)

    let triggerRecovery: () => void = () => {}

    const { getByTestId, unmount } = render(
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
    expect(fetchSpy).toHaveBeenCalledTimes(1)
    unmount()
    expect(fetchSpy).toHaveBeenCalledTimes(1)
  })
})
