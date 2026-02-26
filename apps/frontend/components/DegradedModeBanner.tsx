'use client'

import { useBackendReadiness } from '@/lib/providers/backend-readiness-provider'

/**
 * Full-page banner displayed when the backend is not ready.
 *
 * Gates the entire UI — nothing behind it is interactive.
 * Shows a user-friendly, non-technical message.
 */
export default function DegradedModeBanner({
  children,
}: {
  children: React.ReactNode
}) {
  const { isReady } = useBackendReadiness()

  if (isReady) {
    return <>{children}</>
  }

  return (
    <div className="degraded-mode-overlay">
      <div className="degraded-mode-card">
        <div className="degraded-mode-spinner" aria-hidden="true" />
        <h1 className="degraded-mode-title">We&apos;re getting things ready</h1>
        <p className="degraded-mode-message">
          The service is starting up. This usually takes just a moment.
        </p>
        <p className="degraded-mode-submessage">
          This page will update automatically when everything is ready.
        </p>
      </div>
    </div>
  )
}
