'use client'

import { useEffect, useRef } from 'react'
import { useBackendReadiness } from '@/lib/providers/backend-readiness-provider'

/**
 * Syncs server-known backend status to the client.
 * When the server has already detected the backend is down (e.g. after an RSC
 * backend call failed), rendering this with ready={false} causes the client
 * to enter recovery (banner + polling) on hydration.
 */
interface BackendStatusSyncSignal {
  renderPid: number
  statusVersion: number
  backendMode: string
}

export default function BackendStatusSync({
  ready,
  signal,
}: {
  ready: boolean
  signal?: BackendStatusSyncSignal
}) {
  const { triggerRecovery } = useBackendReadiness()
  const lastReadyRef = useRef<boolean>(ready)
  const lastHandledDownEventRef = useRef<string | null>(null)
  const handledInitialDownRef = useRef(false)

  useEffect(() => {
    const downEventKey =
      signal != null
        ? `pid:${signal.renderPid}:v:${signal.statusVersion}:mode:${signal.backendMode}`
        : null
    const wasReady = lastReadyRef.current
    const becameDown = wasReady && !ready
    const initialDownWithoutTransition =
      !ready && !wasReady && !handledInitialDownRef.current
    const shouldHandleDown =
      !ready &&
      (downEventKey != null
        ? downEventKey !== lastHandledDownEventRef.current
        : becameDown || initialDownWithoutTransition)

    if (shouldHandleDown) {
      if (downEventKey != null) {
        lastHandledDownEventRef.current = downEventKey
      }
      handledInitialDownRef.current = true
      triggerRecovery()
    }
    lastReadyRef.current = ready
  }, [ready, triggerRecovery, signal])

  return null
}
