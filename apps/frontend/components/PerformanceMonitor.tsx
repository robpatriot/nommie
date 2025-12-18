'use client'

import { useEffect } from 'react'

// Performance thresholds (in milliseconds)
const THRESHOLDS = {
  FIRST_PAINT_GOOD: 1000,
  FIRST_PAINT_POOR: 2500,
  FCP_GOOD: 1800,
  FCP_POOR: 3000,
  LCP_GOOD: 2500,
  LCP_POOR: 4000,
} as const

/**
 * Performance Monitor Component
 *
 * Logs detailed performance metrics to the console on page load.
 * Only enabled in development or when PERF_MONITOR=true is set.
 *
 * Usage: Add to your root layout (conditionally in development)
 *
 * <PerformanceMonitor />
 */
export function PerformanceMonitor() {
  useEffect(() => {
    // Only run in browser
    if (typeof window === 'undefined') return

    // Check if monitoring is enabled
    const isEnabled =
      process.env.NODE_ENV === 'development' ||
      process.env.NEXT_PUBLIC_PERF_MONITOR === 'true'

    if (!isEnabled) return

    // Store LCP entry collected via PerformanceObserver
    let lcpEntry: LargestContentfulPaint | null = null

    // Set up PerformanceObserver for LCP (replaces deprecated getEntriesByType)
    let lcpObserver: PerformanceObserver | null = null
    if ('PerformanceObserver' in window) {
      try {
        lcpObserver = new PerformanceObserver((list) => {
          const entries = list.getEntries()
          // The last entry is the final LCP value
          if (entries.length > 0) {
            lcpEntry = entries[entries.length - 1] as LargestContentfulPaint
          }
        })
        lcpObserver.observe({ entryTypes: ['largest-contentful-paint'] })
      } catch {
        // LCP observer not supported - silently ignore
      }
    }

    // Wait for page to fully load
    const logPerformanceMetrics = () => {
      console.group('🚀 Performance Metrics')

      // Navigation Timing
      const navTiming = performance.getEntriesByType(
        'navigation'
      )[0] as PerformanceNavigationTiming

      if (navTiming) {
        const metrics = {
          // DNS
          'DNS Lookup': navTiming.domainLookupEnd - navTiming.domainLookupStart,
          // Connection
          'TCP Connection': navTiming.connectEnd - navTiming.connectStart,
          // TLS (if HTTPS)
          'TLS Negotiation': navTiming.secureConnectionStart
            ? navTiming.connectEnd - navTiming.secureConnectionStart
            : 0,
          // Server Response
          'Time to First Byte (TTFB)':
            navTiming.responseStart - navTiming.requestStart,
          // Download
          'HTML Download Time': navTiming.responseEnd - navTiming.responseStart,
          // Processing
          'DOM Processing': navTiming.domInteractive - navTiming.responseEnd,
          'DOM Complete': navTiming.domComplete - navTiming.domInteractive,
          'Load Event': navTiming.loadEventEnd - navTiming.loadEventStart,
          // Totals
          'Total Load Time': navTiming.loadEventEnd - navTiming.fetchStart,
          'DOM Ready':
            navTiming.domContentLoadedEventEnd - navTiming.fetchStart,
        }

        console.table(
          Object.fromEntries(
            Object.entries(metrics).map(([key, value]) => [
              key,
              `${value.toFixed(2)} ms`,
            ])
          )
        )
      }

      // Paint Timing
      const paintEntries = performance.getEntriesByType('paint')
      if (paintEntries.length > 0) {
        console.group('🎨 Paint Metrics')
        paintEntries.forEach((entry) => {
          const rating = getPaintRating(entry.name, entry.startTime)
          console.log(
            `${rating} ${entry.name}: ${entry.startTime.toFixed(2)} ms`
          )
        })
        console.groupEnd()
      }

      // Resource Timing Summary
      const resources = performance.getEntriesByType(
        'resource'
      ) as PerformanceResourceTiming[]

      if (resources.length > 0) {
        const resourceTypes = resources.reduce(
          (acc, r) => {
            const type = getResourceType(r.name)
            if (!acc[type]) {
              acc[type] = { count: 0, totalSize: 0, totalTime: 0 }
            }
            acc[type].count++
            acc[type].totalTime += r.responseEnd - r.fetchStart
            // Try to get size from transferSize (compressed) or decodedBodySize
            const size = r.transferSize || r.decodedBodySize || 0
            acc[type].totalSize += size
            return acc
          },
          {} as Record<
            string,
            { count: number; totalSize: number; totalTime: number }
          >
        )

        console.group('📦 Resource Loading Summary')
        console.table(
          Object.entries(resourceTypes).map(([type, stats]) => ({
            Type: type,
            Count: stats.count,
            'Total Size': formatBytes(stats.totalSize),
            'Total Time': `${stats.totalTime.toFixed(2)} ms`,
            'Avg Time': `${(stats.totalTime / stats.count).toFixed(2)} ms`,
          }))
        )

        // Show slowest resources
        const slowResources = resources
          .map((r) => ({
            name: r.name.split('/').pop() || r.name,
            fullName: r.name,
            time: r.responseEnd - r.fetchStart,
            size: r.transferSize || r.decodedBodySize || 0,
          }))
          .sort((a, b) => b.time - a.time)
          .slice(0, 5)

        if (slowResources.length > 0) {
          console.group('🐌 Slowest Resources')
          console.table(
            slowResources.map((r) => ({
              Resource: r.name,
              'Load Time': `${r.time.toFixed(2)} ms`,
              Size: formatBytes(r.size),
            }))
          )
          console.groupEnd()
        }

        console.groupEnd()
      }

      // Web Vitals - LCP (using PerformanceObserver instead of deprecated getEntriesByType)
      if (lcpEntry) {
        // LargestContentfulPaint has renderTime or loadTime
        const lcpValue =
          lcpEntry.renderTime ?? lcpEntry.loadTime ?? lcpEntry.startTime ?? 0
        if (lcpValue > 0) {
          const rating = getLcpRating(lcpValue)
          console.log(
            `${rating} Largest Contentful Paint (LCP): ${lcpValue.toFixed(2)} ms`
          )
        }
      }

      console.groupEnd()

      // Clean up observer
      if (lcpObserver) {
        lcpObserver.disconnect()
      }
    }

    // Log immediately if already loaded, otherwise wait for load
    if (document.readyState === 'complete') {
      // Use setTimeout to ensure all performance entries are available
      setTimeout(logPerformanceMetrics, 100)
    } else {
      window.addEventListener('load', () => {
        setTimeout(logPerformanceMetrics, 100)
      })
    }

    // Cleanup function
    return () => {
      if (lcpObserver) {
        lcpObserver.disconnect()
      }
    }
  }, [])

  return null
}

function getPaintRating(name: string, startTime: number): string {
  if (name === 'first-paint') {
    if (startTime < THRESHOLDS.FIRST_PAINT_GOOD) return '✅'
    if (startTime < THRESHOLDS.FIRST_PAINT_POOR) return '⚠️'
    return '❌'
  }
  if (name === 'first-contentful-paint') {
    if (startTime < THRESHOLDS.FCP_GOOD) return '✅'
    if (startTime < THRESHOLDS.FCP_POOR) return '⚠️'
    return '❌'
  }
  return ''
}

function getLcpRating(lcpValue: number): string {
  if (lcpValue < THRESHOLDS.LCP_GOOD) return '✅'
  if (lcpValue < THRESHOLDS.LCP_POOR) return '⚠️'
  return '❌'
}

function getResourceType(url: string): string {
  if (url.match(/\.(js|mjs)$/i)) return 'JavaScript'
  if (url.match(/\.css$/i)) return 'CSS'
  if (url.match(/\.(jpg|jpeg|png|gif|webp|svg|ico)$/i)) return 'Image'
  if (url.match(/\.(woff|woff2|ttf|otf|eot)$/i)) return 'Font'
  if (url.match(/\/api\//)) return 'API'
  return 'Other'
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`
}
