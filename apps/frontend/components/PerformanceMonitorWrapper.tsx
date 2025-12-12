'use client'

import dynamic from 'next/dynamic'

// Dynamically import PerformanceMonitor (only used in development)
// This wrapper is needed because dynamic imports with ssr: false
// cannot be used directly in Server Components in Next.js 16
const PerformanceMonitor = dynamic(
  () =>
    import('@/components/PerformanceMonitor').then((mod) => ({
      default: mod.PerformanceMonitor,
    })),
  { ssr: false }
)

export default function PerformanceMonitorWrapper() {
  return <PerformanceMonitor />
}
