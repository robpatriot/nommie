// apps/frontend/components/PerformanceMonitorWrapper.tsx
'use client'

import { useEffect, useState } from 'react'

// Dev-only wrapper that avoids next/dynamic (which can trigger SSR bailout markers).
export default function PerformanceMonitorWrapper() {
  const [Monitor, setMonitor] = useState<null | React.ComponentType>(null)

  useEffect(() => {
    if (process.env.NODE_ENV !== 'development') return

    let cancelled = false

    ;(async () => {
      try {
        const mod = await import('@/components/PerformanceMonitor')
        if (!cancelled) {
          setMonitor(() => mod.PerformanceMonitor)
        }
      } catch {
        // no-op: monitor is optional in dev
      }
    })()

    return () => {
      cancelled = true
    }
  }, [])

  if (process.env.NODE_ENV !== 'development') return null
  if (!Monitor) return null

  return <Monitor />
}
