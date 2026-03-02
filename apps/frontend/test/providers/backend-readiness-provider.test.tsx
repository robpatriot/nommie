import React from 'react'
import { render, act, waitFor } from '@testing-library/react'
import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import {
  QueryClient,
  QueryClientProvider,
  useMutation,
} from '@tanstack/react-query'
import {
  BackendReadinessProvider,
  useBackendReadiness,
  type CancelPoll,
} from '@/lib/providers/backend-readiness-provider'
import ReadinessQueryObserver from '@/components/ReadinessQueryObserver'
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
    const driver = new ManualPollDriver()

    const { getByTestId } = render(
      <BackendReadinessProvider pollDriver={driver}>
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

  it('enters degraded immediately on first reportFailure("transient") before any success (startup)', () => {
    // Before a baseline is established (no reportSuccess() call yet), even
    // a transient failure is sufficient evidence the backend is down.
    let reportFailure: (kind: 'permanent' | 'transient') => void = () => {}
    const driver = new ManualPollDriver()

    const { getByTestId } = render(
      <BackendReadinessProvider pollDriver={driver}>
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
    expect(getByTestId('readiness').textContent).toBe('not-ready')
  })

  it('after baseline: stays ready on a single transient failure', () => {
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

    // Establish baseline first
    act(() => {
      reportSuccess()
    })
    expect(getByTestId('readiness').textContent).toBe('ready')
    act(() => {
      reportFailure('transient')
    })
    expect(getByTestId('readiness').textContent).toBe('ready')
  })

  it('after baseline: enters degraded after 2 consecutive transient failures', () => {
    let reportFailure: (kind: 'permanent' | 'transient') => void = () => {}
    let reportSuccess: () => void = () => {}
    const driver = new ManualPollDriver()

    const { getByTestId } = render(
      <BackendReadinessProvider pollDriver={driver}>
        <TestConsumer
          onState={(s) => {
            reportFailure = s.reportFailure
            reportSuccess = s.reportSuccess
          }}
        />
      </BackendReadinessProvider>
    )

    act(() => {
      reportSuccess()
    })
    act(() => {
      reportFailure('transient')
    })
    expect(getByTestId('readiness').textContent).toBe('ready')
    act(() => {
      reportFailure('transient')
    })
    expect(getByTestId('readiness').textContent).toBe('not-ready')
  })

  it('after baseline: reportSuccess() resets transient count', () => {
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

    // Establish baseline
    act(() => {
      reportSuccess()
    })
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

  it('does not transition to healthy on reportSuccess(); only /readyz probe successes do', async () => {
    const driver = new ManualPollDriver()
    const fetchSpy = vi.mocked(global.fetch)
    fetchSpy.mockResolvedValue({ ok: false } as Response)

    let reportSuccess: () => void = () => {}
    let triggerRecovery: () => void = () => {}

    const { getByTestId } = render(
      <BackendReadinessProvider pollDriver={driver}>
        <TestConsumer
          onState={(s) => {
            reportSuccess = s.reportSuccess
            triggerRecovery = s.triggerRecovery
          }}
        />
      </BackendReadinessProvider>
    )

    act(() => {
      triggerRecovery()
    })
    expect(getByTestId('readiness').textContent).toBe('not-ready')

    act(() => {
      reportSuccess()
      reportSuccess()
      reportSuccess()
    })
    expect(getByTestId('readiness').textContent).toBe('not-ready')

    fetchSpy.mockResolvedValue({ ok: true } as Response)
    await act(async () => {
      await driver.tick()
    })
    expect(getByTestId('readiness').textContent).toBe('not-ready')
    await act(async () => {
      await driver.tick()
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

  it('resets backoff to fast phase on first probe success during recovery', async () => {
    class CapturingPollDriver extends ManualPollDriver {
      capturedDelays: number[] = []
      schedule(cb: () => void, delayMs: number): CancelPoll {
        this.capturedDelays.push(delayMs)
        return super.schedule(cb, delayMs)
      }
    }

    const driver = new CapturingPollDriver()
    const fetchSpy = vi.mocked(global.fetch)
    fetchSpy.mockResolvedValue({ ok: true } as Response)

    // Control time: start at 0 (fast phase), then jump well past the medium
    // duration threshold (300_000ms in production) into the slow-backoff zone.
    let now = 0
    vi.spyOn(Date, 'now').mockImplementation(() => now)

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

    // Enter degraded while elapsed clock is at 0.
    act(() => {
      triggerRecovery()
    })
    expect(getByTestId('readiness').textContent).toBe('not-ready')

    // Advance well into the slow phase before the first probe fires. Using
    // 400_000ms ensures elapsed > RECOVERY_MEDIUM_DURATION_MS regardless of
    // whether IS_TEST is evaluated at module load with or without the env stub.
    now = 400_000

    // First probe returns ok=true — threshold not met yet (need 2 successes).
    await act(async () => {
      await driver.tick()
    })
    expect(getByTestId('readiness').textContent).toBe('not-ready')

    // With the backoff reset, the confirmatory probe must be scheduled at the
    // fast rate (RECOVERY_FAST_MS = 1_000ms). Without the fix it would be
    // RECOVERY_SLOW_MS = 30_000ms.
    expect(driver.capturedDelays.at(-1)).toBeLessThanOrEqual(1_000)
  })

  it('mutation error triggers reportFailure and enters degraded', async () => {
    const queryClient = new QueryClient({
      defaultOptions: {
        mutations: { retry: 0 },
      },
    })
    function FailingMutationButton() {
      const mutation = useMutation({
        mutationFn: async () => {
          throw new Error('network error')
        },
      })
      return (
        <button
          type="button"
          onClick={() => mutation.mutate()}
          data-testid="trigger-mutation"
        >
          Fail
        </button>
      )
    }
    const driver = new ManualPollDriver()
    const { getByTestId } = render(
      <QueryClientProvider client={queryClient}>
        <BackendReadinessProvider pollDriver={driver}>
          <ReadinessQueryObserver />
          <TestConsumer />
          <FailingMutationButton />
        </BackendReadinessProvider>
      </QueryClientProvider>
    )
    expect(getByTestId('readiness').textContent).toBe('ready')
    await act(async () => {
      getByTestId('trigger-mutation').click()
    })
    await waitFor(() => {
      expect(getByTestId('readiness').textContent).toBe('not-ready')
    })
  })
})
