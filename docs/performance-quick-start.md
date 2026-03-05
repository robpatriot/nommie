# Performance Profiling Quick Start

This is an operational guide for profiling Nommie page-load performance.  
It focuses on the fastest ways to diagnose performance issues during development.

For deeper explanations and additional tools see `performance-profiling-guide.md`.

## Recommended Workflow

1. Run the application using a production build.
2. Use Lighthouse to get a quick automated performance report.
3. Use Chrome DevTools Performance tab to identify bottlenecks.

Production builds are recommended because development builds include additional tooling and are significantly slower than real deployments.

Start services with:

pnpm build  
pnpm start

The frontend will normally run at:

http://localhost:3000

---

## Option 1 — Lighthouse Audit (Fastest)

Run the profiling script:

pnpm perf:profile

Other variants:

pnpm perf:profile:desktop  
pnpm perf:profile:mobile

Custom run:

bash scripts/profile-performance.sh http://localhost:3000 ./performance-reports true

This produces:

- HTML report
- JSON report
- terminal summary

Reports are written to `./performance-reports`.

Key metrics to review:

- Largest Contentful Paint (LCP)
- First Contentful Paint (FCP)
- Total Blocking Time (TBT)
- Cumulative Layout Shift (CLS)

---

## Option 2 — Chrome DevTools Performance Tab

1. Open Chrome DevTools (F12).
2. Select the **Performance** tab.
3. Enable:
   - Network throttling (Fast 3G recommended)
   - CPU throttling (4× slowdown)
   - Disable cache
4. Start recording.
5. Perform a hard reload (Cmd+Shift+R / Ctrl+Shift+R).
6. Stop recording after the page loads.

Look for:

- long tasks (>50 ms)
- slow scripting blocks
- layout or paint bottlenecks

The Bottom-Up panel helps identify which functions consume the most time.

---

## Option 3 — Network Tab

For resource loading analysis:

1. Open **Network** tab.
2. Enable **Disable cache**.
3. Set throttling (Fast 3G recommended).
4. Hard reload the page.

Use the waterfall to identify:

- large resources
- sequential blocking requests
- slow server responses (TTFB)

---

## Quick Profiling Checklist

Before running measurements:

- Use a production build
- Clear browser cache
- Close other browser tabs
- Disable extensions
- Use network throttling
- Run multiple measurements and average results

