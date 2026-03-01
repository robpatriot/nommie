'use client'

import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from 'react'

interface BackendReadinessContextValue {
  /** Whether the backend is confirmed ready. */
  isReady: boolean
  /** Force resume readiness polling if an API request fails. */
  triggerRecovery: () => void
}

const BackendReadinessContext = createContext<BackendReadinessContextValue>({
  isReady: true,
  triggerRecovery: () => {},
})

/** Access backend readiness state from any client component. */
export function useBackendReadiness() {
  return useContext(BackendReadinessContext)
}

// ── Configuration ──────────────────────────────────────────────────

/** Event dispatched when a query/mutation fails with backend-down (suppressed); listener calls triggerRecovery. */
export const BACKEND_RECOVERY_TRIGGER_EVENT = 'nommie:backend-recovery-trigger'

/** Persists across remounts (e.g. Strict Mode) so we stay in recovery and don't re-request ws-token. */
let recoveryTriggeredThisSession = false

const IS_TEST = process.env.NEXT_PUBLIC_FETCH_MODE === 'test'
const PROBE_TIMEOUT_MS = 1_000
const FAILURE_THRESHOLD = 2

// Recovery polling: 1s for 30s, then 5s till 5min, then 30s
const RECOVERY_FAST_MS = IS_TEST ? 10 : 1_000
const RECOVERY_FAST_DURATION_MS = IS_TEST ? 50 : 30_000
const RECOVERY_MEDIUM_MS = IS_TEST ? 20 : 5_000
const RECOVERY_MEDIUM_DURATION_MS = IS_TEST ? 100 : 300_000 // 5 min
const RECOVERY_SLOW_MS = IS_TEST ? 30 : 30_000

function nextRecoveryDelayMs(elapsedMs: number): number {
  if (elapsedMs < RECOVERY_FAST_DURATION_MS) return RECOVERY_FAST_MS
  if (elapsedMs < RECOVERY_MEDIUM_DURATION_MS) return RECOVERY_MEDIUM_MS
  return RECOVERY_SLOW_MS
}

// ── Poll driver ────────────────────────────────────────────────────

export type CancelPoll = { cancel: () => void }

export interface PollDriver {
  /** “Run immediately” in prod. Tests may choose to queue and require manual ticks. */
  run: (cb: () => void) => void
  /** Schedule next poll attempt. */
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
   * Initial ready state from the server (e.g. from layout/page after a backend call failed).
   * Defaults to true (assume BE is up); set false when server already knows BE is down.
   */
  initialReady?: boolean
  /** Injected poll driver for deterministic tests; defaults to real timers in production. */
  pollDriver?: PollDriver
}

export function BackendReadinessProvider({
  children,
  initialReady = true,
  pollDriver = defaultPollDriver,
}: BackendReadinessProviderProps) {
  const [isReady, setIsReady] = useState(
    () => initialReady && !recoveryTriggeredThisSession
  )
  const modeRef = useRef<'healthy' | 'recovering'>(
    initialReady && !recoveryTriggeredThisSession ? 'healthy' : 'recovering'
  )
  const consecutiveFailuresRef = useRef(0)
  const scheduledRef = useRef<CancelPoll | null>(null)
  const recoveryStartedAtRef = useRef<number>(0)

  const clearScheduled = useCallback(() => {
    if (scheduledRef.current) {
      scheduledRef.current.cancel()
      scheduledRef.current = null
    }
  }, [])

  const poll = useCallback(
    async function pollInner() {
      try {
        const response = await fetch('/readyz', {
          method: 'GET',
          signal: AbortSignal.timeout(PROBE_TIMEOUT_MS),
        })

        if (response.ok) {
          recoveryTriggeredThisSession = false
          modeRef.current = 'healthy'
          setIsReady(true)
          clearScheduled()
          return
        }
      } catch {
        consecutiveFailuresRef.current += 1
      }

      const elapsed = Date.now() - recoveryStartedAtRef.current
      const delayMs = nextRecoveryDelayMs(elapsed)

      clearScheduled()
      if (typeof document !== 'undefined' && document.hidden) {
        return
      }
      scheduledRef.current = pollDriver.schedule(() => {
        void pollInner()
      }, delayMs)
    },
    [clearScheduled, pollDriver]
  )

  const triggerRecovery = useCallback(() => {
    if (modeRef.current !== 'healthy') return

    recoveryTriggeredThisSession = true
    modeRef.current = 'recovering'
    setIsReady(false)
    consecutiveFailuresRef.current = FAILURE_THRESHOLD
    recoveryStartedAtRef.current = Date.now()

    clearScheduled()
    pollDriver.run(() => {
      void poll()
    })
  }, [clearScheduled, poll, pollDriver])

  useEffect(() => {
    if (!initialReady) {
      recoveryTriggeredThisSession = true
      recoveryStartedAtRef.current = Date.now()
      consecutiveFailuresRef.current = FAILURE_THRESHOLD
      pollDriver.run(() => {
        void poll()
      })
    }
  }, [initialReady, pollDriver, poll])

  useEffect(() => {
    if (typeof window === 'undefined') return
    const onTrigger = () => triggerRecovery()
    window.addEventListener(BACKEND_RECOVERY_TRIGGER_EVENT, onTrigger)
    return () =>
      window.removeEventListener(BACKEND_RECOVERY_TRIGGER_EVENT, onTrigger)
  }, [triggerRecovery])

  useEffect(() => {
    if (typeof document === 'undefined') return

    const onVisibilityChange = () => {
      if (document.hidden) {
        clearScheduled()
      } else {
        if (modeRef.current === 'recovering') {
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
    <BackendReadinessContext.Provider value={{ isReady, triggerRecovery }}>
      {children}
    </BackendReadinessContext.Provider>
  )
}
