'use client'

import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from 'react'
import { useRouter } from 'next/navigation'
import { drainResponseBody } from '../http/drain-response'

type ReadinessMode = 'healthy' | 'suspect' | 'degraded'

interface BackendReadinessContextValue {
  isReady: boolean
  mode: ReadinessMode
  reportDependencyOutage: () => void
  reportOperationSuccess: () => void
  triggerRecovery: () => void
  markNeedsReconcileAfterRecovery: () => void
  recoveryGeneration: number
}

const BackendReadinessContext = createContext<BackendReadinessContextValue>({
  isReady: true,
  mode: 'healthy',
  reportDependencyOutage: () => {},
  reportOperationSuccess: () => {},
  triggerRecovery: () => {},
  markNeedsReconcileAfterRecovery: () => {},
  recoveryGeneration: 0,
})

/** Access backend readiness state from any client component. */
export function useBackendReadiness() {
  return useContext(BackendReadinessContext)
}

// ── Configuration ──────────────────────────────────────────────────

const PROBE_TIMEOUT_MS = 1_000
const RECOVERY_SUCCESS_THRESHOLD = 2

// Recovery polling: 1s for 30s, then 5s till 5min, then 30s
const RECOVERY_FAST_MS = 1_000
const RECOVERY_FAST_DURATION_MS = 30_000
const RECOVERY_MEDIUM_MS = 5_000
const RECOVERY_MEDIUM_DURATION_MS = 300_000
const RECOVERY_SLOW_MS = 30_000

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
  const [mode, setMode] = useState<ReadinessMode>(
    initialReady ? 'healthy' : 'degraded'
  )
  const isReady = mode === 'healthy'
  const modeRef = useRef<ReadinessMode>(mode)
  const consecutiveSuccessRef = useRef(0)
  const scheduledRef = useRef<CancelPoll | null>(null)
  const recoveryStartedAtRef = useRef<number>(0)
  const mountedRef = useRef(true)
  // Tracks whether a healthy backend has ever been confirmed for this client
  // session. Before the first confirmed success, any failure (even 'transient')
  // immediately enters degraded — there is no established healthy baseline to
  // protect against flapping. Always starts false: optimistic initialReady is
  // an assumption, not confirmation.
  const hasEverSucceededRef = useRef(false)
  const needsReconcileAfterRecoveryRef = useRef(false)
  const [recoveryGeneration, setRecoveryGeneration] = useState(0)
  // True when we mounted in degraded mode (initialReady=false). Used to trigger
  // a route refresh on first recovery so the client gets a fresh RSC payload
  // instead of the one produced while the backend was down (which can leave
  // the tree suspended and never render).
  const startedInDegradedRef = useRef(!initialReady)
  // Set when we transition to healthy after having started in degraded; consumed
  // by useEffect so router.refresh() runs after React has committed (avoids
  // "Rendered more hooks than during the previous render").
  const pendingStartupRecoveryRefreshRef = useRef(false)
  const pendingRecoveryRefreshRef = useRef(false)
  const router = useRouter()

  const clearScheduled = useCallback(() => {
    if (scheduledRef.current) {
      scheduledRef.current.cancel()
      scheduledRef.current = null
    }
  }, [])

  const enterDegraded = useCallback(() => {
    if (modeRef.current === 'degraded') return
    modeRef.current = 'degraded'
    setMode('degraded')
    consecutiveSuccessRef.current = 0
    recoveryStartedAtRef.current = Date.now()
  }, [])

  const enterSuspect = useCallback(() => {
    if (modeRef.current !== 'healthy') return
    modeRef.current = 'suspect'
    setMode('suspect')
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

        // Always drain/cancel the body so the connection can be reused and
        // DevTools/network tooling do not show hanging downloads.
        await drainResponseBody(response)

        if (response.ok) {
          // /readyz 200 is authoritative for recovering from Degraded.
          // Also allow Suspect → Healthy on consecutive 200s to avoid a loop where
          // ws-token 503 triggers Degraded immediately after we recover, causing
          // Degraded → Healthy → ws-token 503 → Degraded → ...
          const inDegradedOrSuspect =
            modeRef.current === 'degraded' || modeRef.current === 'suspect'
          if (inDegradedOrSuspect) {
            const next = consecutiveSuccessRef.current + 1
            consecutiveSuccessRef.current = next
            if (next >= RECOVERY_SUCCESS_THRESHOLD) {
              const prevMode = modeRef.current
              hasEverSucceededRef.current = true
              modeRef.current = 'healthy'
              setMode('healthy')
              clearScheduled()
              if (needsReconcileAfterRecoveryRef.current) {
                needsReconcileAfterRecoveryRef.current = false
              }
              setRecoveryGeneration((g) => g + 1)
              if (prevMode === 'degraded' || prevMode === 'suspect') {
                pendingRecoveryRefreshRef.current = true
              }
              if (startedInDegradedRef.current) {
                startedInDegradedRef.current = false
                pendingStartupRecoveryRefreshRef.current = true
              }
              return
            }
            // A successful probe is a positive signal: reset the elapsed clock so
            // the confirmatory probe fires at the fast rate rather than whichever
            // backoff tier we've drifted into.
            recoveryStartedAtRef.current = Date.now()
          }
        } else {
          consecutiveSuccessRef.current = 0
          if (modeRef.current !== 'degraded') {
            enterDegraded()
          }
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
    [clearScheduled, pollDriver, enterDegraded]
  )

  const startPolling = useCallback(() => {
    clearScheduled()
    pollDriver.run(() => {
      void poll()
    })
  }, [clearScheduled, poll, pollDriver])

  const reportDependencyOutage = useCallback(() => {
    if (modeRef.current === 'healthy') {
      enterSuspect()
      startPolling()
    } else if (modeRef.current === 'suspect') {
      startPolling()
    } else if (modeRef.current === 'degraded') {
      startPolling()
    }
  }, [enterSuspect, startPolling])

  const reportOperationSuccess = useCallback(() => {
    hasEverSucceededRef.current = true
    if (modeRef.current === 'suspect') {
      modeRef.current = 'healthy'
      setMode('healthy')
      clearScheduled()
    }
  }, [clearScheduled])

  const triggerRecovery = useCallback(() => {
    if (modeRef.current !== 'healthy') return
    enterDegraded()
    startPolling()
  }, [enterDegraded, startPolling])

  const markNeedsReconcileAfterRecovery = useCallback(() => {
    needsReconcileAfterRecoveryRef.current = true
  }, [])

  useEffect(() => {
    mountedRef.current = true
    return () => {
      mountedRef.current = false
      clearScheduled()
    }
  }, [clearScheduled])

  // Run router.refresh() after we've transitioned to healthy from startup-degraded,
  // so it runs in a separate commit and avoids hook-order issues when the tree expands.
  useEffect(() => {
    if (
      mode === 'healthy' &&
      (pendingStartupRecoveryRefreshRef.current ||
        pendingRecoveryRefreshRef.current)
    ) {
      const startupRecovery = pendingStartupRecoveryRefreshRef.current
      pendingStartupRecoveryRefreshRef.current = false
      pendingRecoveryRefreshRef.current = false
      if (startupRecovery) {
        // Force a full document navigation to current URL after startup recovery.
        // This guarantees a fresh server route resolution (including redirects)
        // and avoids app-router pending segment wedges after degraded startup.
        window.location.replace(window.location.href)
      } else {
        router.refresh()
      }
    }
  }, [mode, router])

  useEffect(() => {
    if (!initialReady) {
      recoveryStartedAtRef.current = Date.now()
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
      value={{
        isReady,
        mode,
        reportDependencyOutage,
        reportOperationSuccess,
        triggerRecovery,
        markNeedsReconcileAfterRecovery,
        recoveryGeneration,
      }}
    >
      {children}
    </BackendReadinessContext.Provider>
  )
}
