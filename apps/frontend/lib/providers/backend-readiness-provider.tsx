'use client'

import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from 'react'

export type FailureKind = 'permanent' | 'transient'

interface BackendReadinessContextValue {
  isReady: boolean
  reportFailure: (kind: FailureKind) => void
  reportSuccess: () => void
  triggerRecovery: () => void
}

const BackendReadinessContext = createContext<BackendReadinessContextValue>({
  isReady: true,
  reportFailure: () => {},
  reportSuccess: () => {},
  triggerRecovery: () => {},
})

/** Access backend readiness state from any client component. */
export function useBackendReadiness() {
  return useContext(BackendReadinessContext)
}

// ── Configuration ──────────────────────────────────────────────────

const IS_TEST = process.env.NEXT_PUBLIC_FETCH_MODE === 'test'
const PROBE_TIMEOUT_MS = 1_000
const TRANSIENT_FAILURE_THRESHOLD = 2
const RECOVERY_SUCCESS_THRESHOLD = 2

// Recovery polling: 1s for 30s, then 5s till 5min, then 30s
const RECOVERY_FAST_MS = IS_TEST ? 10 : 1_000
const RECOVERY_FAST_DURATION_MS = IS_TEST ? 50 : 30_000
const RECOVERY_MEDIUM_MS = IS_TEST ? 20 : 5_000
const RECOVERY_MEDIUM_DURATION_MS = IS_TEST ? 100 : 300_000
const RECOVERY_SLOW_MS = IS_TEST ? 30 : 30_000

function nextRecoveryDelayMs(elapsedMs: number): number {
  if (elapsedMs < RECOVERY_FAST_DURATION_MS) return RECOVERY_FAST_MS
  if (elapsedMs < RECOVERY_MEDIUM_DURATION_MS) return RECOVERY_MEDIUM_MS
  return RECOVERY_SLOW_MS
}

// ── Poll driver ────────────────────────────────────────────────────

export type CancelPoll = { cancel: () => void }

export interface PollDriver {
  run: (cb: () => void) => void
  schedule: (cb: () => void, delayMs: number) => CancelPoll
}

const defaultPollDriver: PollDriver = {
  run: (cb) => cb(),
  schedule: (cb, delayMs) => {
    const id = setTimeout(cb, delayMs)
    return { cancel: () => clearTimeout(id) }
  },
}

// ── Provider ───────────────────────────────────────────────────────

interface BackendReadinessProviderProps {
  children: React.ReactNode
  /**
   * Initial ready state from the server (e.g. from layout when backend is known down).
   * Defaults to true (optimistic: assume BE is up).
   */
  initialReady?: boolean
  pollDriver?: PollDriver
}

export function BackendReadinessProvider({
  children,
  initialReady = true,
  pollDriver = defaultPollDriver,
}: BackendReadinessProviderProps) {
  const [isReady, setIsReady] = useState(initialReady)
  const modeRef = useRef<'healthy' | 'degraded'>(
    initialReady ? 'healthy' : 'degraded'
  )
  const consecutiveTransientRef = useRef(0)
  const consecutiveSuccessRef = useRef(0)
  const scheduledRef = useRef<CancelPoll | null>(null)
  const recoveryStartedAtRef = useRef<number>(0)
  const mountedRef = useRef(true)

  const clearScheduled = useCallback(() => {
    if (scheduledRef.current) {
      scheduledRef.current.cancel()
      scheduledRef.current = null
    }
  }, [])

  const enterDegraded = useCallback(() => {
    if (modeRef.current === 'degraded') return
    modeRef.current = 'degraded'
    setIsReady(false)
    consecutiveSuccessRef.current = 0
    recoveryStartedAtRef.current = Date.now()
  }, [])

  const poll = useCallback(
    async function pollInner() {
      if (!mountedRef.current) return

      try {
        const response = await fetch('/readyz', {
          method: 'GET',
          signal: AbortSignal.timeout(PROBE_TIMEOUT_MS),
        })

        if (response.ok) {
          const next = consecutiveSuccessRef.current + 1
          consecutiveSuccessRef.current = next
          if (next >= RECOVERY_SUCCESS_THRESHOLD) {
            modeRef.current = 'healthy'
            setIsReady(true)
            clearScheduled()
            return
          }
        } else {
          consecutiveSuccessRef.current = 0
        }
      } catch {
        consecutiveSuccessRef.current = 0
      }

      if (!mountedRef.current) return
      const elapsed = Date.now() - recoveryStartedAtRef.current
      const delayMs = nextRecoveryDelayMs(elapsed)
      if (typeof document !== 'undefined' && document.hidden) return

      clearScheduled()
      scheduledRef.current = pollDriver.schedule(() => {
        void pollInner()
      }, delayMs)
    },
    [clearScheduled, pollDriver]
  )

  const startPolling = useCallback(() => {
    clearScheduled()
    pollDriver.run(() => {
      void poll()
    })
  }, [clearScheduled, poll, pollDriver])

  const reportFailure = useCallback(
    (kind: FailureKind) => {
      if (kind === 'permanent') {
        enterDegraded()
        startPolling()
        return
      }
      const next = consecutiveTransientRef.current + 1
      consecutiveTransientRef.current = next
      if (next >= TRANSIENT_FAILURE_THRESHOLD) {
        enterDegraded()
        startPolling()
      }
    },
    [enterDegraded, startPolling]
  )

  const reportSuccess = useCallback(() => {
    consecutiveTransientRef.current = 0
  }, [])

  const triggerRecovery = useCallback(() => {
    if (modeRef.current !== 'healthy') return
    enterDegraded()
    startPolling()
  }, [enterDegraded, startPolling])

  useEffect(() => {
    mountedRef.current = true
    return () => {
      mountedRef.current = false
      clearScheduled()
    }
  }, [clearScheduled])

  useEffect(() => {
    if (!initialReady) {
      enterDegraded()
      startPolling()
    }
  }, [initialReady, enterDegraded, startPolling])

  useEffect(() => {
    if (typeof document === 'undefined') return

    const onVisibilityChange = () => {
      if (document.hidden) {
        clearScheduled()
      } else {
        if (modeRef.current === 'degraded') {
          recoveryStartedAtRef.current = Date.now()
          clearScheduled()
          pollDriver.run(() => {
            void poll()
          })
        }
      }
    }

    document.addEventListener('visibilitychange', onVisibilityChange)
    return () => {
      document.removeEventListener('visibilitychange', onVisibilityChange)
      clearScheduled()
    }
  }, [clearScheduled, poll, pollDriver])

  return (
    <BackendReadinessContext.Provider
      value={{ isReady, reportFailure, reportSuccess, triggerRecovery }}
    >
      {children}
    </BackendReadinessContext.Provider>
  )
}
