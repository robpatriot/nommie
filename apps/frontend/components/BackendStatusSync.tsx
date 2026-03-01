'use client'

import { useEffect } from 'react'
import { useBackendReadiness } from '@/lib/providers/backend-readiness-provider'

/**
 * Syncs server-known backend status to the client.
 * When the server has already detected the backend is down (e.g. after an RSC
 * backend call failed), rendering this with ready={false} causes the client
 * to enter recovery (banner + polling) on hydration.
 */
export default function BackendStatusSync({ ready }: { ready: boolean }) {
  const { triggerRecovery } = useBackendReadiness()

  useEffect(() => {
    if (!ready) {
      triggerRecovery()
    }
  }, [ready, triggerRecovery])

  return null
}
