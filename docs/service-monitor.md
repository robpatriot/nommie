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

Both Frontend (FE) and Backend (BE) expose four standardized health endpoints.

| Endpoint | Purpose | Publicity | Behavior |
|---|---|---|---|
| `/healthz` | Liveness Probe | Public | Returns `200 OK` if the process is running. |
| `/readyz` | Readiness Probe | Public | Returns `200 OK` only if all dependencies are ready. Returns `503 Service Unavailable` otherwise. |
| `/internal/healthz` | Rich Liveness | Internal | Rich JSON including service name and uptime. |
| `/internal/readyz` | Rich Readiness | Internal | Rich JSON including state mode, dependency statuses, and migration state. |

> [!NOTE]
> All health responses include `Cache-Control: no-store` to prevent stale caching by proxies or browsers.

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

The frontend monitors the backend's `/readyz` status. If the backend is not ready:

1.  **Readiness Probe:** The frontend `/readyz` returns `503`.
2.  **UI Gate:** A full-page **Degraded Mode Banner** is displayed, blocking all user 
    interaction and navigation.
3.  **Polling:** The frontend continues to poll the backend readiness endpoint in the 
    background and automatically clears the banner once the backend returns to `healthy`.
4.  **WebSocket Protection:** The WebSocket connection is suspended or delayed until 
    backend readiness is confirmed.

---

## 🧭 Related Documents

- `architecture-overview.md` — High-level system shape.
- `backend-error-handling.md` — RFC 7807 mapping and error strategy.
- `project-milestones.md` — Milestone 25 implementation details.
