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

const IS_TEST = process.env.NEXT_PUBLIC_FETCH_MODE === 'test'
const STARTUP_POLL_MS = IS_TEST ? 10 : 2_000
const RECOVERY_BASE_MS = IS_TEST ? 10 : 2_000
const RECOVERY_MAX_MS = IS_TEST ? 50 : 30_000
const FAILURE_THRESHOLD = 2
const RECOVERY_THRESHOLD = 2

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
  /** Injected poll driver for deterministic tests; defaults to real timers in production. */
  pollDriver?: PollDriver
}

export function BackendReadinessProvider({
  children,
  pollDriver = defaultPollDriver,
}: BackendReadinessProviderProps) {
  const [isReady, setIsReady] = useState(false)

  // Track mode internally (not exposed to consumers — they only see isReady)
  const modeRef = useRef<'startup' | 'healthy' | 'recovering'>('startup')
  const consecutiveSuccessesRef = useRef(0)
  const consecutiveFailuresRef = useRef(0)
  const attemptRef = useRef(0)
  const scheduledRef = useRef<CancelPoll | null>(null)

  const clearScheduled = useCallback(() => {
    if (scheduledRef.current) {
      scheduledRef.current.cancel()
      scheduledRef.current = null
    }
  }, [])

  const poll = useCallback(
    async function pollInner() {
      try {
        const backendBase = process.env.NEXT_PUBLIC_BACKEND_BASE_URL
        if (!backendBase) {
          return
        }

        const response = await fetch(`${backendBase}/readyz`, {
          method: 'GET',
          signal: AbortSignal.timeout(5000),
        })

        if (response.ok) {
          consecutiveSuccessesRef.current += 1
          consecutiveFailuresRef.current = 0

          if (
            modeRef.current !== 'healthy' &&
            consecutiveSuccessesRef.current >= RECOVERY_THRESHOLD
          ) {
            modeRef.current = 'healthy'
            attemptRef.current = 0
            setIsReady(true)
            // Stop polling – we're healthy
            clearScheduled()
            return
          }

          if (modeRef.current === 'healthy') {
            // Already healthy and still healthy – stop polling
            clearScheduled()
            return
          }
        } else {
          consecutiveFailuresRef.current += 1
          consecutiveSuccessesRef.current = 0

          if (
            modeRef.current === 'healthy' &&
            consecutiveFailuresRef.current >= FAILURE_THRESHOLD
          ) {
            modeRef.current = 'recovering'
            setIsReady(false)
          }
        }
      } catch {
        consecutiveFailuresRef.current += 1
        consecutiveSuccessesRef.current = 0

        if (
          modeRef.current === 'healthy' &&
          consecutiveFailuresRef.current >= FAILURE_THRESHOLD
        ) {
          modeRef.current = 'recovering'
          setIsReady(false)
        }
      }

      // Schedule next poll
      const interval =
        modeRef.current === 'startup'
          ? STARTUP_POLL_MS
          : Math.min(
              RECOVERY_BASE_MS * Math.pow(2, attemptRef.current),
              RECOVERY_MAX_MS
            )

      attemptRef.current += 1

      clearScheduled()
      scheduledRef.current = pollDriver.schedule(() => {
        void pollInner()
      }, interval)
    },
    [clearScheduled, pollDriver]
  )

  const triggerRecovery = useCallback(() => {
    // Only trigger if we think we're healthy.
    // If we're already startup/recovering, the poll loop is already running.
    if (modeRef.current === 'healthy') {
      modeRef.current = 'recovering'
      setIsReady(false)
      consecutiveFailuresRef.current = FAILURE_THRESHOLD
      consecutiveSuccessesRef.current = 0
      attemptRef.current = 0

      clearScheduled()
      pollDriver.run(() => {
        void poll()
      })
    }
  }, [clearScheduled, poll, pollDriver])

  useEffect(() => {
    // Start initial poll immediately (via driver.run for test determinism)
    pollDriver.run(() => {
      void poll()
    })

    return () => {
      clearScheduled()
    }
  }, [clearScheduled, poll, pollDriver])

  return (
    <BackendReadinessContext.Provider value={{ isReady, triggerRecovery }}>
      {children}
    </BackendReadinessContext.Provider>
  )
}
