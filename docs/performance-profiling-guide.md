# Performance Profiling Guide

This document describes tools and techniques for analyzing page-load performance in Nommie.

Unlike architectural documentation, this file is an operational reference and may contain commands, examples, and step-by-step procedures.

## Page Load Timeline

A typical full page reload contains several phases:

1. Navigation start
2. Network connection and request
3. Server processing
4. HTML parsing and DOM construction
5. Resource loading
6. First paint and content rendering
7. Page becoming interactive

Understanding which phase dominates load time is the goal of performance profiling.

---

## Chrome DevTools Performance Tab

The Performance tab provides the most detailed view of browser activity.

Typical setup:

- enable network throttling
- enable CPU throttling
- disable cache
- perform a hard reload during recording

Important panels:

Timeline  
Shows when network requests, rendering, and scripting occur.

Flame chart  
Shows how JavaScript execution time is distributed.

Bottom-up view  
Highlights which functions consume the most processing time.

Main thread view  
Identifies long tasks that block page interaction.

---

## Chrome DevTools Network Tab

The Network tab focuses on resource loading.

Typical analysis workflow:

1. Disable cache.
2. Apply network throttling.
3. Hard reload the page.
4. Inspect the waterfall chart.

Important timing phases:

Queued  
DNS lookup  
Connection establishment  
Time to First Byte (TTFB)  
Content download

Common issues identified here include:

- large resources
- render-blocking CSS or JavaScript
- slow backend responses

Network requests can be exported as HAR files for further analysis.

---

## Lighthouse Audits

Lighthouse performs automated performance analysis.

Using DevTools:

1. Open the Lighthouse panel.
2. Select the Performance category.
3. Run the audit.

Typical metrics produced:

- First Contentful Paint
- Largest Contentful Paint
- Total Blocking Time
- Cumulative Layout Shift
- Speed Index

Lighthouse also highlights optimization opportunities such as:

- unused JavaScript
- large images
- render-blocking resources

---

## Web Vitals Monitoring

Web Vitals measure real user experience.

Key metrics:

LCP — loading performance  
FID — responsiveness  
CLS — visual stability  
FCP — first visible content  
TTFB — server response time

These metrics can be logged during development or sent to monitoring systems in production.

---

## Performance API

The browser Performance API exposes timing information programmatically.

It can be used to inspect:

- navigation timing
- resource loading times
- paint timing
- long tasks on the main thread

This is useful for automated logging or internal performance dashboards.

---

## Performance Monitor

Chrome includes a real-time performance monitor.

Open DevTools → More Tools → Performance Monitor.

Metrics shown include:

- CPU usage
- JavaScript heap size
- DOM node count
- event listener count

This tool is useful while interacting with the page to detect runtime performance issues.

---

## Measurement Best Practices

For reliable measurements:

Use production builds  
Run multiple tests and average results  
Test under network throttling  
Use CPU throttling to simulate slower devices  
Test on mobile devices when possible

Always compare results before and after changes to measure impact.

---

## Tools Overview

Performance Tab  
Deep browser activity analysis.

Network Tab  
Resource loading and request timing.

Lighthouse  
Automated performance audits.

Performance API  
Programmatic access to timing data.

Web Vitals  
User-experience performance metrics.

Performance Monitor  
Real-time browser metrics while interacting with the page.

