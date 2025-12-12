# Performance Profiling Quick Start

Quick guide to get started with performance profiling for Nommie.

## Option 1: Use the Performance Monitor Component (Development)

Add the `PerformanceMonitor` component to your root layout for automatic console logging in development.

**Note**: This component works in both dev and production builds. However, for accurate performance metrics that reflect what users experience, use a production build (`pnpm build && pnpm start`). Dev builds are slower and not representative of production performance, but can be useful for quick checks during development.

### Add to `app/layout.tsx`:

```typescript
import { PerformanceMonitor } from '@/components/PerformanceMonitor'

export default async function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  // ... existing code ...
  
  return (
    <html lang="en" data-theme={initialResolved} data-user-theme={initialTheme}>
      <body className={`${inter.className} tabletop-shell`}>
        {/* ... existing code ... */}
        {process.env.NODE_ENV === 'development' && <PerformanceMonitor />}
        {/* ... rest of layout ... */}
      </body>
    </html>
  )
}
```

This will automatically log detailed performance metrics to the console on every page load in development mode.

**To enable in production**, set `NEXT_PUBLIC_PERF_MONITOR=true` in your environment variables.

## Option 2: Run Lighthouse Audit (Automated)

**⚠️ Prerequisites:** Your frontend (and backend if needed) must be running before running Lighthouse!

**📦 Build Type Note:** For accurate performance metrics, use a production build (`pnpm build && pnpm start`). Dev builds (`pnpm start` or `pnpm fe:up`) work but will show slower, less accurate metrics. See the [full guide](./performance-profiling-guide.md#3-dev-build-vs-production-build) for details.

1. Start your services (preferably production build):
   ```bash
   # Production build (recommended for accurate metrics)
   pnpm build
   pnpm start
   
   # Or dev build (faster iteration, less accurate metrics)
   # pnpm start
   # Or separately:
   # pnpm fe:up
   # pnpm be:up
   ```

2. Wait for services to be ready (frontend typically on http://localhost:3000)

3. Run Lighthouse audit:
   ```bash
   # Basic usage (mobile preset, default URL: http://localhost:3000)
   pnpm perf:profile

   # Desktop preset
   pnpm perf:profile:desktop

   # Mobile preset (explicit)
   pnpm perf:profile:mobile

   # Custom URL and output directory
   bash scripts/profile-performance.sh http://localhost:3000 ./my-reports true
   ```

The script will:
- Run Lighthouse audit
- Generate HTML and JSON reports
- Open the HTML report in your browser
- Display key metrics in the terminal

Reports are saved to `./performance-reports/` by default.

## Option 3: Chrome DevTools (Manual, Most Detailed)

For the most detailed analysis:

1. **Open Chrome DevTools** (F12 or Cmd+Option+I)
2. **Go to Performance tab**
3. **Configure:**
   - Enable "Network" throttling (Fast 3G or Slow 3G)
   - Enable "CPU" throttling (4x slowdown)
   - Check "Disable cache"
   - Check "Enable advanced paint instrumentation"
4. **Record:**
   - Click Record button
   - Hard reload: Cmd+Shift+R (Mac) or Ctrl+Shift+R (Windows/Linux)
   - Wait for page to fully load
   - Stop recording
5. **Analyze:**
   - Check the timeline for bottlenecks
   - Look for long tasks (red triangles)
   - Review the Bottom-Up panel for expensive operations

See the [full performance profiling guide](./performance-profiling-guide.md) for detailed instructions.

## Option 4: Network Tab Analysis

For resource loading analysis:

1. **Open Chrome DevTools** → **Network tab**
2. **Configure:**
   - Check "Disable cache"
   - Set throttling (Fast 3G recommended)
3. **Clear and reload:**
   - Clear current requests
   - Hard reload (Cmd+Shift+R / Ctrl+Shift+R)
4. **Analyze:**
   - Waterfall view shows loading sequence
   - Check TTFB (Time to First Byte) for each resource
   - Identify blocking resources
   - Export as HAR for further analysis

## Quick Performance Checklist

Before profiling, ensure:

- [ ] **Services are running** (frontend and backend if needed)
- [ ] **Use production build** (`pnpm build && pnpm start`) for accurate metrics
  - Dev builds work but give slower, non-representative results
  - Only use dev builds if you need source-level debugging
- [ ] Clear browser cache
- [ ] Close other tabs/extensions
- [ ] Use throttling for realistic conditions
- [ ] Run multiple times and average results

See the [Performance Profiling Checklist](./performance-profiling-guide.md#performance-profiling-checklist) in the full guide for a comprehensive checklist.

## Key Metrics to Watch

### Critical Metrics (Web Vitals)

- **LCP (Largest Contentful Paint)**: < 2.5s (good)
- **FID (First Input Delay)**: < 100ms (good)
- **CLS (Cumulative Layout Shift)**: < 0.1 (good)

### Load Time Breakdown

- **TTFB (Time to First Byte)**: < 600ms (good)
- **FCP (First Contentful Paint)**: < 1.8s (good)
- **Total Load Time**: < 3s (good)

### Resource Metrics

- **Initial JS Bundle**: < 200KB (good)
- **Total Resources**: < 50 requests (good)
- **Total Page Weight**: < 2MB (good)

## Next Steps

1. Run a baseline measurement with one of the methods above
2. Document your findings
3. Identify bottlenecks (see full guide)
4. Make optimizations
5. Re-measure to validate improvements

For comprehensive details, see the [full performance profiling guide](./performance-profiling-guide.md).

