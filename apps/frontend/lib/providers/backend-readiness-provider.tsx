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
const RECOVERY_BASE_MS = IS_TEST ? 10 : 2_000
const RECOVERY_MAX_MS = IS_TEST ? 50 : 30_000
const PROBE_TIMEOUT_MS = 1_000
const FAILURE_THRESHOLD = 2

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
  const [isReady, setIsReady] = useState(true)
  const modeRef = useRef<'healthy' | 'recovering'>('healthy')
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
        const response = await fetch('/readyz', {
          method: 'GET',
          signal: AbortSignal.timeout(PROBE_TIMEOUT_MS),
        })

        if (response.ok) {
          modeRef.current = 'healthy'
          attemptRef.current = 0
          setIsReady(true)
          clearScheduled()
          return
        }
      } catch {
        consecutiveFailuresRef.current += 1
      }

      const interval = Math.min(
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
    if (modeRef.current !== 'healthy') return

    modeRef.current = 'recovering'
    setIsReady(false)
    consecutiveFailuresRef.current = FAILURE_THRESHOLD
    attemptRef.current = 0

    clearScheduled()
    pollDriver.run(() => {
      void poll()
    })
  }, [clearScheduled, poll, pollDriver])

  useEffect(() => {
    return () => {
      clearScheduled()
    }
  }, [clearScheduled])

  return (
    <BackendReadinessContext.Provider value={{ isReady, triggerRecovery }}>
      {children}
    </BackendReadinessContext.Provider>
  )
}
