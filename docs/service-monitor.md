# 🔍 Nommie — Service Monitoring & Readiness

## Document Scope

This document describes the robustness and readiness system implemented for the Nommie 
project. It covers liveness and readiness probes, dependency monitoring, 
and the graceful degraded mode for the frontend.

---

## 🌐 Overview

Nommie uses a production-grade readiness gating system to ensure that both the backend 
and frontend services are fully operational before accepting user traffic. This prevents 
cascading failures and ensures that users only interact with a "Healthy" system.

The system distinguishes between **Liveness** (process is alive) and **Readiness** 
(service is ready to handle requests).

---

## 📡 Endpoint Architecture

**Frontend** (root, for browser and public probing):

| Endpoint | Purpose | Behavior |
|---|---|---|
| `GET /livez` | Liveness | Returns `200 OK` if the process is up. |
| `GET /readyz` | Readiness (aggregate) | Returns `200 OK` when app is ready (e.g. backend ready); `503` otherwise. |

**Backend** (under `/api`, externally routed):

| Endpoint | Purpose | Behavior |
|---|---|---|
| `GET /api/livez` | Liveness | Returns `200 OK` if the process is up. |
| `GET /api/readyz` | Readiness | Returns `200 OK` when deps/migrations/monitor are ready; `503` otherwise. This is the **canonical public** readiness endpoint (routed by Caddy). |
| `GET /api/internal/readyz` | Readiness (rich) | **Backend-only, not exposed via Caddy.** Same status codes as `/api/readyz`; richer JSON for humans/devops. Intended for internal/devops use only (e.g. direct backend access). |

> [!NOTE]
> All health responses include `Cache-Control: no-store` to prevent stale caching by proxies or browsers.
>
> **Internal endpoints:** `/api/internal/*` (including `/api/internal/readyz`) are **not** routed by Caddy and are not publicly accessible. The public readiness endpoint is `/api/readyz`.

---

## ⚙️ Backend Readiness State Machine

The backend maintains a thread-safe state machine (`ReadinessManager`) that tracks the 
following modes:

| Mode | Description | Readiness Status |
|---|---|---|
| **`startup`** | Default state at boot. Waiting for first check successes. | Not Ready |
| **`healthy`** | All dependencies OK and migrations applied. | **Ready** |
| **`recovering`** | Transient failure detected. Polling to recover. | Not Ready |
| **`failed`** | Hard failure (e.g., migrations failed). Permanent state. | Not Ready |

---

## 📉 Failure & Recovery Thresholds

To prevent flapping and unnecessary restarts, the system uses threshold-based transitions:

### Failure Detection
*   **Hard Failures:** (e.g., Database Migration failure). Immediate transition to `failed` mode (1 failure).
*   **Transient Failures:** (e.g., Network drop, Redis restart). Transition to `recovering` mode after **2 consecutive failures**.

### Recovery Logic
*   The system polls failing dependencies with **exponential backoff** (starting at 1s, capped at 30s).
*   Transition back to `healthy` mode requires **2 consecutive successes** from ALL dependencies.

---

## 🧪 Frontend Degraded Mode

The frontend assumes the backend is healthy on startup (optimistic). Degraded mode triggers after a real failed API request (network error, timeout, 5xx, 503); both query and mutation failures are observed (via React Query QueryCache and MutationCache). When degraded:

1.  **Readiness Probe:** The frontend aggregate `/readyz` returns `503`.
2.  **UI Gate:** A full-page **Degraded Mode Banner** is displayed, blocking all user 
    interaction and navigation.
3.  **Polling:** The frontend polls `GET /readyz` (its aggregate endpoint, 1s timeout), which checks backend readiness; it exits degraded after **2 consecutive** successful probe responses.
4.  **WebSocket Protection:** The WebSocket connection is suspended or delayed until 
    backend readiness is confirmed.

---

## 🧭 Related Documents

- `architecture-overview.md` — High-level system shape.
- `backend-error-handling.md` — RFC 7807 mapping and error strategy.
- `project-milestones.md` — Milestone 25 implementation details.
