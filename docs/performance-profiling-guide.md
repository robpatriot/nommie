# Performance Profiling Guide for Full Page Reloads

This guide covers the optimum ways to profile full page reload performance and understand where time is spent during page loads.

## Overview: The Full Page Load Timeline

A full page reload consists of several phases:

1. **Navigation Start** → **Redirect** (if any)
2. **Fetch Start** → DNS lookup, TCP connection, TLS negotiation
3. **Response Start** → Server processing time
4. **DOM Content Loaded** → HTML parsing, DOM construction
5. **Load Complete** → All resources loaded, page interactive
6. **First Paint** → First pixels rendered
7. **First Contentful Paint (FCP)** → First content visible
8. **Largest Contentful Paint (LCP)** → Main content rendered
9. **Time to Interactive (TTI)** → Page fully interactive

---

## Method 1: Chrome DevTools Performance Tab (Recommended)

The **Performance tab** provides the most detailed view of where time is spent.

### Setup for Accurate Measurement

1. **Open DevTools** (F12 or Cmd+Option+I)
2. **Go to Performance tab**
3. **Before recording:**
   - Enable "Network" throttling: Set to "Slow 3G" or "Fast 3G" to simulate real conditions
   - Enable "CPU" throttling: 4x slowdown (Chrome 58+)
   - Check "Disable cache" for cold load testing
   - Check "Enable advanced paint instrumentation" (for paint timing)

4. **Record:**
   - Click the **Record** button (or press Cmd+E / Ctrl+E)
   - Do a **hard reload**: Cmd+Shift+R (Mac) or Ctrl+Shift+R (Windows/Linux)
   - Wait for page to fully load
   - Click **Stop**

### What to Look For

#### Timeline View (Waterfall)
- **Purple bars**: HTML document download
- **Blue bars**: JavaScript files
- **Green bars**: CSS files
- **Orange bars**: Images
- **Gray bars**: Other resources

#### Flame Chart
- **Yellow (Scripting)**: JavaScript execution time
- **Purple (Rendering)**: Layout/paint operations
- **Green (Painting)**: Paint operations
- **Gray (System)**: Browser overhead
- **Light blue (Loading)**: Network requests
- **Dark blue (Idle)**: Waiting time

#### Bottom-Up / Call Tree
- Shows which functions consume the most time
- Sort by "Self Time" to find expensive operations
- Expand to see call stacks

#### Main Thread Activity
- Long tasks (>50ms) block interactivity
- Look for red warning triangles
- Identify render-blocking resources

### Key Metrics to Extract

1. **Total Load Time**: From navigationStart to loadEventEnd
2. **DOMContentLoaded**: Time to parse HTML and build DOM
3. **First Paint**: First pixel rendered
4. **First Contentful Paint**: First text/image rendered
5. **Largest Contentful Paint**: Main content visible
6. **Time to Interactive**: Page fully responsive

---

## Method 2: Chrome DevTools Network Tab

The **Network tab** shows resource loading timing in detail.

### Setup

1. Open **Network tab**
2. Ensure **"Disable cache"** is checked
3. Set throttling to desired speed
4. Clear current requests
5. **Hard reload** (Cmd+Shift+R / Ctrl+Shift+R)

### Resource Timing Breakdown

For each resource, you can see:

- **Queued**: Time waiting to start
- **DNS Lookup**: DNS resolution time
- **Initial Connection**: TCP handshake + TLS negotiation
- **SSL**: TLS handshake (if HTTPS)
- **Request Sent**: Time to send request
- **Waiting (TTFB)**: Time to First Byte (server processing)
- **Content Download**: Time to download response body

### Key Insights

- **Waterfall visualization**: Shows parallel vs sequential loading
- **Request initiators**: See what triggers each request
- **Resource size vs time**: Identify large resources
- **Blocking resources**: Render-blocking CSS/JS

### Export Timing Data

1. Right-click in Network tab → "Save all as HAR with content"
2. Use HAR viewer or parse programmatically for analysis

---

## Method 3: Lighthouse (Automated Analysis)

**Lighthouse** provides automated performance audits with actionable recommendations.

### Using Lighthouse in DevTools

1. Open DevTools → **Lighthouse** tab
2. Select **Performance** category
3. Choose device type: **Desktop** or **Mobile**
4. Check **"Clear storage"** for accurate cold load
5. Click **"Analyze page load"**
6. Do a hard reload when prompted

### Using Lighthouse CLI

```bash
# Install Lighthouse CLI
npm install -g lighthouse

# Run performance audit
lighthouse http://localhost:3000 --only-categories=performance --output=html --output-path=./lighthouse-report.html

# Generate JSON for programmatic analysis
lighthouse http://localhost:3000 --only-categories=performance --output=json --output-path=./lighthouse-report.json
```

### Key Metrics from Lighthouse

- **First Contentful Paint (FCP)**: < 1.8s (good)
- **Largest Contentful Paint (LCP)**: < 2.5s (good)
- **Total Blocking Time (TBT)**: < 200ms (good)
- **Cumulative Layout Shift (CLS)**: < 0.1 (good)
- **Speed Index**: < 3.4s (good)

Lighthouse also identifies:
- Render-blocking resources
- Unused CSS/JS
- Large images
- Slow server response times

---

## Method 4: Web Vitals API (Real User Monitoring)

**Web Vitals** provides standardized metrics for real user monitoring.

### Implementation

Add to your Next.js app (e.g., `app/layout.tsx` or a client component):

```typescript
// lib/web-vitals.ts
export function reportWebVitals(metric: {
  id: string
  name: string
  value: number
  rating: 'good' | 'needs-improvement' | 'poor'
  delta: number
  entries: PerformanceEntry[]
}) {
  // Log to console in development
  if (process.env.NODE_ENV === 'development') {
    console.log('[Web Vitals]', metric)
  }

  // Send to analytics service in production
  // Example: send to your backend
  if (process.env.NODE_ENV === 'production') {
    fetch('/api/analytics', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(metric),
    }).catch(() => {
      // Ignore errors to avoid impacting performance
    })
  }
}
```

### Next.js Built-in Web Vitals

Next.js has built-in support for Web Vitals. Create `app/_app.tsx` or add to your root layout:

```typescript
// For Pages Router: app/_app.tsx
// export { reportWebVitals } from 'next/web-vitals'

// For App Router: Use a client component
// app/web-vitals.tsx
'use client'
import { useEffect } from 'react'
import { onCLS, onFID, onFCP, onLCP, onTTFB } from 'web-vitals'

export function WebVitals() {
  useEffect(() => {
    onCLS(console.log)
    onFID(console.log)
    onFCP(console.log)
    onLCP(console.log)
    onTTFB(console.log)
  }, [])
  return null
}
```

### Key Web Vitals Metrics

- **LCP (Largest Contentful Paint)**: Loading performance
- **FID (First Input Delay)**: Interactivity
- **CLS (Cumulative Layout Shift)**: Visual stability
- **FCP (First Contentful Paint)**: Perceived load speed
- **TTFB (Time to First Byte)**: Server response time

---

## Method 5: Performance API (Programmatic Access)

The **Performance API** provides detailed timing data programmatically.

### Navigation Timing API

```typescript
// Get full page load timing
const perfData = performance.getEntriesByType('navigation')[0] as PerformanceNavigationTiming

console.log('DNS:', perfData.domainLookupEnd - perfData.domainLookupStart, 'ms')
console.log('TCP:', perfData.connectEnd - perfData.connectStart, 'ms')
console.log('TLS:', perfData.secureConnectionStart 
  ? perfData.connectEnd - perfData.secureConnectionStart 
  : 0, 'ms')
console.log('TTFB:', perfData.responseStart - perfData.requestStart, 'ms')
console.log('Download:', perfData.responseEnd - perfData.responseStart, 'ms')
console.log('DOM Processing:', perfData.domInteractive - perfData.responseEnd, 'ms')
console.log('Load Complete:', perfData.loadEventEnd - perfData.loadEventStart, 'ms')
console.log('Total Load Time:', perfData.loadEventEnd - perfData.fetchStart, 'ms')
```

### Resource Timing API

```typescript
// Get timing for all resources
const resources = performance.getEntriesByType('resource') as PerformanceResourceTiming[]

resources.forEach(resource => {
  console.log(resource.name, {
    DNS: resource.domainLookupEnd - resource.domainLookupEnd,
    Connection: resource.connectEnd - resource.connectStart,
    TTFB: resource.responseStart - resource.requestStart,
    Download: resource.responseEnd - resource.responseStart,
    Total: resource.responseEnd - resource.fetchStart,
  })
})
```

### Paint Timing API

```typescript
// Get paint timing
const paintEntries = performance.getEntriesByType('paint')

paintEntries.forEach(entry => {
  console.log(entry.name, entry.startTime, 'ms')
  // 'first-paint' or 'first-contentful-paint'
})
```

### Long Task API

```typescript
// Detect long tasks (blocking operations > 50ms)
if ('PerformanceObserver' in window) {
  const observer = new PerformanceObserver((list) => {
    for (const entry of list.getEntries()) {
      console.warn('Long task detected:', entry.duration, 'ms', entry)
    }
  })
  observer.observe({ entryTypes: ['longtask'] })
}
```

### Create a Performance Monitoring Hook

```typescript
// hooks/usePerformanceMetrics.ts
'use client'
import { useEffect } from 'react'

export function usePerformanceMetrics() {
  useEffect(() => {
    if (typeof window === 'undefined') return

    // Navigation timing
    const navTiming = performance.getEntriesByType('navigation')[0] as PerformanceNavigationTiming
    if (navTiming) {
      console.table({
        'DNS Lookup': `${(navTiming.domainLookupEnd - navTiming.domainLookupStart).toFixed(2)} ms`,
        'TCP Connection': `${(navTiming.connectEnd - navTiming.connectStart).toFixed(2)} ms`,
        'TLS Negotiation': navTiming.secureConnectionStart
          ? `${(navTiming.connectEnd - navTiming.secureConnectionStart).toFixed(2)} ms`
          : 'N/A',
        'Time to First Byte': `${(navTiming.responseStart - navTiming.requestStart).toFixed(2)} ms`,
        'Download Time': `${(navTiming.responseEnd - navTiming.responseStart).toFixed(2)} ms`,
        'DOM Processing': `${(navTiming.domInteractive - navTiming.responseEnd).toFixed(2)} ms`,
        'Total Load Time': `${(navTiming.loadEventEnd - navTiming.fetchStart).toFixed(2)} ms`,
      })
    }

    // Resource timing summary
    const resources = performance.getEntriesByType('resource') as PerformanceResourceTiming[]
    const totalResourceTime = resources.reduce(
      (sum, r) => sum + (r.responseEnd - r.fetchStart),
      0
    )
    console.log(`Total resource load time: ${totalResourceTime.toFixed(2)} ms (${resources.length} resources)`)

    // Paint timing
    const paintEntries = performance.getEntriesByType('paint')
    paintEntries.forEach(entry => {
      console.log(`${entry.name}: ${entry.startTime.toFixed(2)} ms`)
    })
  }, [])
}
```

---

## Method 6: Chrome Performance Monitor

The **Performance Monitor** shows real-time metrics as you interact with the page.

1. Open DevTools → **More tools** → **Performance monitor**
2. Metrics shown:
   - CPU usage
   - JS heap size
   - DOM nodes
   - Event listeners
   - Documents (iframes)

---

## Best Practices for Accurate Measurements

### 1. Test Conditions

- **Cold load**: Clear cache, hard reload (Cmd+Shift+R)
- **Warm load**: Regular reload (to test cache effectiveness)
- **Network throttling**: Use "Slow 3G" or "Fast 3G" for realistic conditions
- **CPU throttling**: Enable 4x slowdown for mobile simulation

### 2. Multiple Runs

- Run tests **3-5 times** and average results
- First load is often slower (DNS, connections)
- Ignore outliers

### 3. Dev Build vs Production Build

**None of the profiling methods require a dev build**, but the choice matters:

#### When to Use Production Builds (Recommended)

**Use production builds (`next build && next start`) for accurate performance metrics because:**

- **Accurate representation**: Production builds reflect what users actually experience
- **Optimized code**: Minified, tree-shaken, optimized bundles
- **Proper caching**: Production caching behavior
- **Real bundle sizes**: Actual file sizes shipped to users
- **Server optimizations**: Production Next.js optimizations enabled

**All profiling methods benefit from production builds for accurate results:**
- ✅ Lighthouse - More accurate scores and recommendations
- ✅ Performance Tab - Real-world timing data
- ✅ Network Tab - Actual resource sizes and loading patterns
- ✅ Performance API - Accurate metrics matching production
- ✅ Web Vitals - Real user experience metrics
- ✅ Performance Monitor Component - Production-like measurements

#### When Dev Builds Can Be Useful

**Dev builds (`next dev`) can be helpful for:**

- **Debugging**: Source maps make it easier to identify which source code files are slow
- **Development workflow**: Quick iteration without rebuilding
- **Code-level analysis**: Performance tab can show readable function names instead of minified code
- **Performance Monitor Component**: Convenient console logging during development

**However, dev build metrics are NOT representative because:**
- ❌ Much slower due to hot module reloading overhead
- ❌ Unoptimized bundles (larger file sizes)
- ❌ Different caching behavior
- ❌ Development-only code included
- ❌ No minification or tree-shaking
- ❌ Different webpack/Next.js behavior

**Recommendation**: Use production builds for profiling. Use dev builds only when you need source-level debugging information, then switch to production for accurate measurements.

### 4. Test on Real Devices

- Desktop Chrome performance differs from mobile
- Use Chrome DevTools device emulation
- Test on actual mobile devices when possible

### 5. Compare Before/After

- Baseline measurements before optimizations
- Measure impact of each change
- Track improvements over time

---

## Performance Profiling Checklist

Use this checklist when profiling full page reloads to ensure consistent, accurate measurements.

### Pre-Profiling Setup

- [ ] **Clear browser cache** (Cmd+Shift+Delete / Ctrl+Shift+Delete)
- [ ] **Close other tabs** to minimize system resource usage
- [ ] **Disable browser extensions** that might interfere (ad blockers, etc.)
- [ ] **Use incognito/private mode** for clean profile
- [ ] **Stop background processes** that might affect CPU/memory

### Browser DevTools Configuration

- [ ] **Performance Tab:**
  - [ ] Enable "Network" throttling (Fast 3G or Slow 3G)
  - [ ] Enable "CPU" throttling (4x slowdown)
  - [ ] Check "Disable cache"
  - [ ] Check "Enable advanced paint instrumentation"

- [ ] **Network Tab:**
  - [ ] Check "Disable cache"
  - [ ] Set appropriate throttling

### Test Conditions

- [ ] **Cold Load Test:**
  - [ ] Clear cache
  - [ ] Hard reload (Cmd+Shift+R / Ctrl+Shift+R)
  - [ ] Record from navigation start

- [ ] **Warm Load Test:**
  - [ ] Regular reload (Cmd+R / Ctrl+R)
  - [ ] Test cache effectiveness

- [ ] **Multiple Runs:**
  - [ ] Run 3-5 times
  - [ ] Average the results
  - [ ] Note any outliers

### Environment

- [ ] **Production Build:**
  - [ ] `pnpm fe:build` to build frontend production bundle
  - [ ] `pnpm start` runs backend production (`cargo run --release`) and frontend production server (`next start`)
  - [ ] `NODE_ENV=production` (automatically set by Next.js in production mode)

- [ ] **Network Conditions:**
  - [ ] Test on real network (not localhost if possible)
  - [ ] Test with throttling enabled
  - [ ] Test on actual mobile device (if applicable)

### Measurement Points

Document these metrics for each run:

#### Navigation Timing
- [ ] DNS Lookup time
- [ ] TCP Connection time
- [ ] TLS Negotiation time
- [ ] Time to First Byte (TTFB)
- [ ] HTML Download time
- [ ] DOM Processing time
- [ ] Total Load Time

#### Paint Metrics
- [ ] First Paint (FP)
- [ ] First Contentful Paint (FCP)
- [ ] Largest Contentful Paint (LCP)

#### Resource Loading
- [ ] Number of resources
- [ ] Total resource size
- [ ] Total resource load time
- [ ] Slowest resources (top 5)
- [ ] Blocking resources

#### JavaScript
- [ ] Initial bundle size
- [ ] Total JavaScript size
- [ ] JavaScript execution time
- [ ] Long tasks (>50ms)

#### Layout & Rendering
- [ ] Cumulative Layout Shift (CLS)
- [ ] Time to Interactive (TTI)
- [ ] Total Blocking Time (TBT)

### Analysis

- [ ] **Identify bottlenecks:**
  - [ ] Slow server response (TTFB > 600ms)
  - [ ] Large bundle sizes (>200KB initial JS)
  - [ ] Render-blocking resources
  - [ ] Long JavaScript tasks
  - [ ] Large images
  - [ ] Too many requests

- [ ] **Compare before/after:**
  - [ ] Baseline measurements recorded
  - [ ] Changes documented
  - [ ] Improvements quantified

- [ ] **Document findings:**
  - [ ] Screenshots of Performance tab
  - [ ] Lighthouse reports saved
  - [ ] Key metrics logged
  - [ ] Recommendations noted

### Tools Used

- [ ] Chrome DevTools Performance tab
- [ ] Chrome DevTools Network tab
- [ ] Lighthouse audit
- [ ] Performance API metrics
- [ ] Web Vitals (if implemented)

### Follow-up Actions

- [ ] Create optimization tickets
- [ ] Prioritize improvements
- [ ] Re-test after changes
- [ ] Monitor production metrics (if available)

---

## Understanding Where Time is Spent

### Typical Breakdown (Good Performance)

- **DNS + Connection**: 50-200ms
- **TTFB (Server)**: 100-500ms
- **HTML Download**: 10-100ms
- **DOM Processing**: 100-300ms
- **Resource Loading**: 500-2000ms (parallel)
- **JavaScript Execution**: 200-1000ms
- **Layout/Paint**: 50-200ms

### Red Flags

- **TTFB > 600ms**: Slow server or network
- **Long tasks > 50ms**: Heavy JavaScript execution
- **Blocking resources**: CSS/JS blocking render
- **Large bundle size**: > 200KB initial JS
- **Too many requests**: > 50 resources
- **No parallelization**: Sequential resource loading

---

## Next.js-Specific Optimizations

### 1. Enable Production Build Analysis

```bash
# Analyze bundle size
ANALYZE=true pnpm build

# Or add to package.json
"build:analyze": "ANALYZE=true next build"
```

### 2. Check Build Output

```bash
pnpm build
```

Look for:
- Route sizes
- First Load JS size
- Shared JS size
- Pages with large bundles

### 3. Use Next.js Image Optimization

```typescript
import Image from 'next/image'

// Automatically optimizes images
<Image src="/hero.jpg" width={800} height={600} alt="Hero" />
```

### 4. Enable Compression

Next.js compresses by default in production, but verify headers:
- `Content-Encoding: gzip` or `br`

### 5. Check Server-Side Rendering Performance

- Profile server-side code separately
- Use React Server Components effectively
- Minimize data fetching in components

---

## Recommended Profiling Workflow

1. **Lighthouse Audit**: Get initial score and recommendations
2. **Performance Tab**: Deep dive into timeline and identify bottlenecks
3. **Network Tab**: Analyze resource loading patterns
4. **Performance API**: Add monitoring for production
5. **Web Vitals**: Track real user metrics
6. **Iterate**: Make optimizations and re-measure

---

## Tools Summary

| Tool | Use Case | Granularity | Real User Data |
|------|----------|-------------|----------------|
| **Performance Tab** | Deep analysis, identify bottlenecks | Millisecond | No |
| **Network Tab** | Resource loading analysis | Millisecond | No |
| **Lighthouse** | Automated audit, recommendations | Second | No |
| **Performance API** | Programmatic monitoring | Millisecond | Yes (with code) |
| **Web Vitals** | Standardized metrics | Millisecond | Yes (with code) |

---

## Additional Resources

- [Web.dev Performance](https://web.dev/performance/)
- [Chrome DevTools Performance](https://developer.chrome.com/docs/devtools/performance/)
- [Web Vitals](https://web.dev/vitals/)
- [MDN Performance API](https://developer.mozilla.org/en-US/docs/Web/API/Performance)
- [Next.js Performance](https://nextjs.org/docs/app/building-your-application/optimizing)

