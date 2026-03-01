# 🗺️ Nommie — Milestone & Enhancement Roadmap

## Document Scope

Tracks delivery milestones, optional enhancements, and outstanding work items.
Use this alongside `../dev-roadmap.md` for UI sequencing and
`architecture-overview.md` for the technical baseline each milestone builds on.

This document outlines Nommie's development path:
Core milestones first, then optional and enhancement tracks that can be implemented at any time.

---

## Core Milestones

### ✅ **1. Repository & Project Bootstrap**
**Dependencies:** none  
- **Monorepo Setup:**  
  *Acceptance:* Hello-world frontend and backend build locally.  
  *Status:* ✅ Complete — Monorepo with `apps/frontend`, `apps/backend`, and `packages/`. Root `.env` is canonical; frontend `.env.local` mirrors only `NEXT_PUBLIC_*`.
- **Linting & Formatting:**  
  *Acceptance:* Lint and format hooks pass.  
  *Status:* ✅ Complete — ESLint/Prettier configured for the frontend. Pre-commit hooks active. Scripts: `backend:fmt` → `cargo fmt --manifest-path apps/backend/Cargo.toml --all`; `backend:clippy` → `cargo clippy --manifest-path apps/backend/Cargo.toml --all-targets --all-features -- -D warnings`.

---
 
### ✅ **2. Docker-First Development Environment**
**Dependencies:** 1  
- **Docker Compose Setup:**  
  *Acceptance:* `pnpm start` starts frontend and backend; frontend communicates with backend.  
  *Status:* ✅ Complete — Docker Compose with Postgres (roles, DBs, grants). Host-pnpm for speed; backend runs on host or in container.
- **Postgres TLS/SSL Support:**  
  *Acceptance:* Postgres reachable with TLS.  
  *Status:* ✅ Complete — Postgres connections use TLS with `verify-full` default; shared Postgres TLS image with build-time certificate generation; separate volume for certificates.
 
---
 
### ✅ **3. Database Schema via Init SQL (Scaffolding Only)**
**Dependencies:** 2  
- **Schema Management:**  
  *Acceptance:* Tests bootstrap schema cleanly; `_test` guard enforced.  
  *Status:* ✅ Complete — Single `init.sql` is source of truth; test harness applies schema to `_test` database at startup with guard.
 
---
 
### ✅ **4. Testing Harness & Policies**
**Dependencies:** 3  
- **Test Infrastructure:**  
  *Acceptance:* Tests pass locally and in CI.  
  *Status:* ✅ Complete — `pnpm test` runs unit, integration, and smoke tests; Actix in-process integration harness; first smoke test `create → add AI → snapshot`.
 
---
 
### ✅ **5. Error Shapes & Logging**
**Dependencies:** 4  
- **Problem Details Format:**  
  *Acceptance:* Consistent error responses; logs include `trace_id`.  
  *Status:* ✅ Complete — Problem Details error format `{ type, title, status, detail, code, trace_id }`; `code` uses SCREAMING_SNAKE convention; middleware assigns a `trace_id` per request.
 
---
 
### ✅ **6. Database Schema (Actual Entities)**
**Dependencies:** 3, 4  
- **Entity Definitions:**  
  *Acceptance:* Schema applies cleanly and aligns with game lifecycle.  
  *Status:* ✅ Complete — Entities defined in `init.sql`: `users`, `games`, `memberships`, `bids`, `plays`, `scores`; enums for game and membership states; foreign keys and indexes added; AI players represented in `users` table like humans.
 
---
 
### ✅ **7. User Authentication**
**Dependencies:** 6  
- **OAuth & JWT:**  
  *Acceptance:* Users authenticate via Google; JWT validation works end-to-end.  
  *Status:* ✅ Complete — Google OAuth for login and account creation; JWTs for frontend/backend authentication; authentication extractor validates JWT and resolves current user.
 
---
 
### ✅ **8. Transactional Tests & DB Access Pattern**
**Dependencies:** 4  
- **Transaction Management:**  
  *Acceptance:* All handlers use `with_txn`; no direct `state.db` usage; lint and tests clean.  
  *Status:* ✅ Complete — Unified request-path DB access through `with_txn`; rollback-by-default test policy; nested `with_txn` behavior defined and tested.
- **Determinism Tools:**  
  *Acceptance:* Tests are reproducible with deterministic time injection.  
  *Status:* ✅ Complete — Injectable clock, seeded RNG, and mock time for reproducible tests.
 
---
 
### ✅ **9. Extractors**
**Dependencies:** 5, 6, 7  
- **Extractor Implementation:**  
  *Acceptance:* Handlers are thin; extractor tests pass; single DB hit for user and membership; input validation consistent across all handlers.  
  *Status:* ✅ Complete — Implemented `AuthToken`, `JwtClaims`, `CurrentUser`, `GameId`, `GameMembership`, and `ValidatedJson<T>`.
- **Extractor Unification:**  
  *Acceptance:* Input validation consistent across all handlers.  
  *Status:* ✅ Complete — All routes use `ValidatedJson<T>`, `AuthToken`, `CurrentUser`, `GameId`, and `GameMembership` consistently.
 
---
 
### ✅ **10. Backend Domain Modules**
**Dependencies:** 7  
- **Pure Domain Logic:**  
  *Acceptance:* `grep` shows no ORM usage in domain code.  
  *Status:* ✅ Complete — Pure logic modules: `rules`, `bidding`, `tricks`, `scoring`, `state`; no SeaORM in domain modules.
 
---
 
### ✅ **11. Frontend App Router Seed**
**Dependencies:** 5, 7  
- **Next.js App Router:**  
  *Acceptance:* Users can authenticate and reach lobby/game views with real data and actions wired end-to-end.  
  *Status:* ✅ Complete — Next.js App Router with server components/actions, guarded by backend JWT resolution; authenticated layout with shared header, theme provider, suspense-loading states; lobby and game routes backed by live data fetching and server mutations.
 
---
 
### ✅ **12. Game Lifecycle (Happy Path)**
**Dependencies:** 9, 7, 10, 11  
- **Complete Game Flow:**  
  *Acceptance:* A full happy-path game completes successfully with deterministic tests guarding regressions.  
  *Status:* ✅ Complete — Complete flow `create → join → ready → deal → bid → trump → tricks → scoring → next round`; integration test covers minimal end-to-end loop.
 
---
 
### ✅ **13. AI Orchestration**
**Dependencies:** 11  
- **AI Automation:**  
  *Acceptance:* Full AI-only games complete successfully; orchestration tests cover bidding, trump selection, trick play, and auto-start flows.  
  *Status:* ✅ Complete — AI performs bidding and legal plays; game advances automatically until human input is required.
 
---
 
### ✅ **14. Validation, Edge Cases, and Property Tests**
**Dependencies:** 11  
- **Error Handling:**  
  *Acceptance:* Error cases handled consistently.  
  *Status:* ✅ Complete — Invalid bids/plays return proper Problem Details.
- **Property Tests:**  
  *Acceptance:* All properties hold across generated games; invariants verified for dealing, progression, scoring, bidding, and serialization.  
  *Status:* ✅ Complete — Property tests confirm trick/scoring invariants; extended property tests verify correctness across generated games.
 
---
 
### ✅ **15. Frontend UX Pass (Round 1)**
**Dependencies:** 11, 13  
- **Core Game UI:**  
  *Acceptance:* Gameplay readable and intuitive.  
  *Status:* ✅ Complete — Hand display, trick area, bidding UI, trump selector; frontend shows Problem Details errors clearly.
 
---
 
### ✅ **16. Frontend UX Pass (Round 2)**
**Dependencies:** 15  
- **Design v1 Implementation:**  
  *Acceptance:* Core screens match design reference.  
  *Status:* ✅ Complete — First-endorsed product design applied across Nommie (typography, spacing, components).
- **Game Config vs Play UI Split:**  
  *Acceptance:* Users transition smoothly between config and play areas.  
  *Status:* ✅ Complete — Game experience split into configuration surface and in-game play surface.
- **Last Trick UI:**  
  *Acceptance:* Previous trick reviewable.  
  *Status:* ✅ Complete — Most recent trick persisted as compact card row so play can continue immediately.
- **User Options:**  
  *Acceptance:* Account preferences persist.  
  *Status:* ✅ Complete — Per-account settings (theme, gameplay preferences) surfaced via profile/options view.
- **Card Play Confirmation Toggle:**  
  *Acceptance:* Card confirmation toggle works.  
  *Status:* ✅ Complete — Per-account option for confirming card plays before submission implemented.
 
---
 
### ✅ **17. Mobile Design & UI Implementation**
**Dependencies:** 11, 15, 16  
- **Design System Parity:**  
  *Acceptance:* Navigation, theming, and interactions feel native.  
  *Status:* ✅ Complete — Mobile-specific design kit defined mirroring web while honoring platform conventions and accessibility.
- **Expo App Foundations:**  
  *Acceptance:* Mobile users can authenticate.  
  *Status:* ✅ Complete — `apps/mobile` Expo client scaffolded with navigation, theming, and auth hand-off using existing backend JWT flow.
- **End-to-End Screens:**  
  *Acceptance:* Mobile users can configure games and play full rounds with UX parity to the web client.  
  *Status:* ✅ Complete — Lobby list, game configuration, and in-game play surfaces implemented with responsive layouts, gestures, and haptics.
- **State & Sync:**  
  *Acceptance:* The app handles reconnects and snapshot refreshes gracefully.  
  *Status:* ✅ Complete — Shared types/API wrapper reused; offline/foreground-resume states with snapshot hydration supported.
 
---
 
### ✅ **18. Architecture & Reliability**
- **WebSockets / Server Push & Architecture:**  
  *Acceptance:* WebSocket sync is the primary update mechanism for active game clients; architecture and testing strategy documented and enforced via automated tests.  
  *Status:* ✅ Complete — Polling replaced with WebSockets; registry/broker split; Redis pub/sub fan-out; end-to-end WebSocket integration tests; architecture documented in `docs/websocket-design.md`.
- **Deployment Stub:**  
  *Acceptance:* Application runs in a minimal production-style configuration.  
  *Status:* ✅ Complete — Minimal production-style environment including FE, BE, DB, and Redis in `docker/prod` with TLS and Caddy reverse proxy.
- **Race-Safe `ensure_user`:**  
  *Acceptance:* No duplicate users/credentials under concurrency.  
  *Status:* ✅ Complete — Non-aborting upserts (`ON CONFLICT DO NOTHING`) with follow-up SELECT; concurrency regression test proves correctness.
 
---
 
### ✅ **19. Observability & Stability**
**Dependencies:** 5, 11  
- **Trace Context Enrichment:**  
  *Acceptance:* Logs actionable; trace ID visible end-to-end.  
  *Status:* ✅ Complete — Logs include `trace_id`, `user_id`, and `game_id` when relevant via instrumented spans.
- **Frontend Trace Display:**  
  *Acceptance:* Trace ID visible end-to-end.  
  *Status:* ✅ Complete — Frontend displays `trace_id` on error surfaces via toast and error boundary components.
- **Health Endpoint:**  
  *Acceptance:* Endpoint returns up/down status with trace context.  
  *Status:* ✅ Complete — Health/readiness probes under `/api/livez`, `/api/readyz` (public), `/api/internal/readyz` (backend-only, not exposed via Caddy); frontend `/livez`, `/readyz`.
- **Security Logging:**  
  *Acceptance:* Security events logged with appropriate detail.  
  *Status:* ✅ Complete — Structured security logging for authentication failures and rate limit hits.
 
---
 
### ✅ **20. Internationalisation & Frontend Localisation**
**Dependencies:** 11, 15  
- **End-to-End Frontend Internationalisation:**  
  *Acceptance:* Frontend UI text fully localized and driven by error codes and message keys rather than inline strings; adding/updating locales is a data change.  
  *Status:* ✅ Complete — Code-driven i18n strategy using `next-intl` with locale-aware request handling, message loading, and per-namespace translation files.
- **Error Localisation via Codes:**  
  *Acceptance:* Error codes mapped to localized messages.  
  *Status:* ✅ Complete — Backend exposes stable `code` values; frontend maps codes via single source of truth (`i18n/errors.ts` + `errors.*` namespaces).
- **Coverage & Tooling:**  
  *Acceptance:* I18n consistency enforced in lint pipeline.  
  *Status:* ✅ Complete — All interactive flows use translations; multiple locales kept in sync via `i18n-check` and `i18n-unused` scripts.
- **Error Code Enforcement:**  
  *Acceptance:* Error code coverage verified automatically.  
  *Status:* ✅ Complete — i18n check script verifies all `KNOWN_ERROR_CODES` have translation keys; frontend warns on unknown codes.
- **Systematic Date/Number Formatting:**  
  *Acceptance:* Date and number formatting respects user locale preferences.  
  *Status:* ✅ Complete — Locale-aware formatting implemented using `Intl` APIs via centralized utilities.
 
---

 
---
 
### ✅ **21. Email Allowlist & Access Control**
**Dependencies:** 7  
- **Email Allowlist:**  
  *Acceptance:* Only emails on the allowlist can sign up or log in.  
  *Status:* ✅ Complete — Email allowlist implemented for signup and login to restrict access to authorised email addresses.
- **Backend Implementation:**  
  *Acceptance:* Configuration is documented and testable.  
  *Status:* ✅ Complete — Allowlist validation integrated into authentication flow with configurable environment variables.
- **Frontend Error Handling:**  
  *Acceptance:* Denied access triggers appropriate error handling and sign-out.  
  *Status:* ✅ Complete — Frontend handles allowlist errors gracefully with sign-out flow when access is denied.
- **StateBuilder Integration:**  
  *Acceptance:* Configuration is consistent.  
  *Status:* ✅ Complete — Allowlist ownership managed through StateBuilder for consistent configuration.
- **Documentation:**  
  *Acceptance:* Email allowlist documented.  
  *Status:* ✅ Complete — Email allowlist configuration documented with environment variable setup.
 
---
 
### ✅ **22. Security Hardening**
**Dependencies:** 2, 7  
- **Docker Image Hardening:**  
  *Acceptance:* Containers run as non-root.  
  *Status:* ✅ Complete — Non-root users and pinned base images for backend and frontend containers.
- **Security Headers:**  
  *Acceptance:* Security headers present.  
  *Status:* ✅ Complete — Content Security Policy (CSP), Permissions-Policy, and X-XSS-Protection headers implemented.
- **Rate Limiting:**  
  *Acceptance:* Rate limiting functional.  
  *Status:* ✅ Complete — Rate limiting middleware implemented with security-specific logging for rate limit hits.
- **CORS Configuration:**  
  *Acceptance:* CORS properly configured.  
  *Status:* ✅ Complete — Tightened CORS configuration for backend API.
- **Environment Validation:**  
  *Acceptance:* Environment validation in place.  
  *Status:* ✅ Complete — Startup validation for critical backend environment variables.
- **Authentication Security:**  
  *Acceptance:* Authentication lifetimes adjusted.  
  *Status:* ✅ Complete — JWT lifetime adjustments; NextAuth session lifetime shortened; security vulnerabilities addressed.
- **Upload Limits:**  
  *Acceptance:* Upload limits enforced.  
  *Status:* ✅ Complete — Universal upload limits implemented.
- **Error Security:**  
  *Acceptance:* No information leakage in errors.  
  *Status:* ✅ Complete — Avoid leaking user existence in forbidden user errors.
- **Connection Security:**  
  *Acceptance:* Credentials handled securely in connection URLs.  
  *Status:* ✅ Complete — Postgres credentials percent-encoded in connection URLs.
 
---
 
### 🚀 **23. WebSocket Architecture Refactor & Realtime Foundations**
**Dependencies:** 10, 18, 23  
- **Generic WebSocket Sessions:**  
  *Acceptance:* Connections no longer tied to `{game_id}` routes.  
  *Status:* ✅ Complete — Single `WsSession` decoupled from games and URLs.
- **Explicit Subscription Model:**  
  *Acceptance:* Connections may dynamically change subscriptions.  
  *Status:* ✅ Complete — Clients explicitly `subscribe` / `unsubscribe` to topics via JSON protocol.
- **User-Based Registration:**  
  *Acceptance:* Backend can broadcast to all active connections for a user.  
  *Status:* ✅ Complete — Connections registered under authenticated `user_id` for user-scoped delivery.
- **Topic-Based Routing:**  
  *Acceptance:* No implicit routing via URLs.  
  *Status:* ✅ Complete — Game events routed only to explicitly subscribed connections.
- **Registry & Responsibility Split:**  
  *Acceptance:* No game logic in generic session or hub layers.  
  *Status:* ✅ Complete — Clear separation between session lifecycle, routing registries, and game-specific realtime logic.
- **Explicit JSON Protocol:**  
  *Acceptance:* All WS communication conforms to the defined protocol.  
  *Status:* ✅ Complete — Typed client commands and server events with documented semantics.
- **User-Scoped Events:**  
  *Acceptance:* Backend can notify users across games.  
  *Status:* ✅ Complete — User-targeted events (e.g. `your_turn`) delivered independently of game subscriptions.
- **Your-Turn Suppression:**  
  *Acceptance:* No redundant `your_turn` for in-game connections; client-side dedupe remains defensive fallback.  
  *Status:* ✅ Complete — Suppress `your_turn` for connections already subscribed to the relevant game.
- **WS Token Strategy:**  
  *Acceptance:* WS connections authenticate using short-lived tokens.  
  *Status:* ✅ Complete — Short-lived WS tokens minted via `/api/ws/token` and presented on upgrade.
- **Subscription Authorisation:**  
  *Acceptance:* Invalid subscriptions rejected with explicit errors and no data leakage.  
  *Status:* ✅ Complete — Server-side validation ensures users may only subscribe to authorised games.
- **Broker Generalisation:**  
  *Acceptance:* Cross-instance realtime delivery supports multiple event types.  
  *Status:* ✅ Complete — Redis Pub/Sub supports both game-scoped and user-scoped realtime events.
- **Authoritative Game State Delivery:**  
  *Acceptance:* Clients receive only authoritative, versioned game state.  
  *Status:* ✅ Complete — State delivered only for subscribed games with monotonic versioning.
- **Version Source Guarantee:**  
  *Acceptance:* Backend guarantees monotonic versions; client dedupe semantics correct.  
  *Status:* ✅ Complete — `game_state.version` sourced from `games.version` with explicit increment semantics.
- **Client Version Reset Semantics:**  
  *Acceptance:* Safe resets on game change; reconnect behaviour defined.  
  *Status:* ✅ Complete — `wsVersionRef` behaviour explicit and resilient to UI refactors.
- **WebSocket Test Suite Update:**  
  *Acceptance:* Lifecycle, routing, protocol, and reconnect behaviour covered.  
  *Status:* ✅ Complete — Tests updated for generic connections and subscriptions.
- **Explicit Behavioural Guarantees:**  
  *Acceptance:* No data before subscribe; ordering guarantees asserted.  
  *Status:* ✅ Complete — Tests assert behaviour rather than incidental timing.
- **Acknowledgement Semantics:**  
  *Acceptance:* Ack messages clearly identify the command being acknowledged.  
  *Status:* ✅ Complete — Acks machine-correlatable with `command` and `topic` fields.
- **Dedicated WS Token Type:**  
  *Acceptance:* `/ws` rejects non-WS tokens; tests prove access tokens cannot open WS connections.  
  *Status:* ⬜ Planned — Enforce WS-only token type distinct from access tokens.
 
---

### 🚀 **24. Startup Readiness, Dependency Health, and Degraded Mode**
**Dependencies:** 18, 19  
- **Health & Readiness Endpoints:**  
  *Acceptance:* Public endpoints expose only up/down status; internal endpoints expose full diagnostic state.  
  *Status:* ✅ Complete — `/api/livez`, `/api/readyz` (public), `/api/internal/readyz` (backend-only) in BE; frontend `/livez`, `/readyz` (aggregate).
- **Dependency Mapping & Enforcement:**  
  *Acceptance:* `ready` remains false until all required dependencies are confirmed.  
  *Status:* ✅ Complete — `ReadinessManager` tracks Postgres and Redis status with failure/success thresholds.
- **Migration-Gated Readiness:**  
  *Acceptance:* Migration failure results in persistent not-ready state and `503` for normal API routes.  
  *Status:* ✅ Complete — Backend stays alive on migration failure but remains in `Failed` mode; `ReadinessGate` middleware returns 503.
- **Frontend Degraded Mode:**  
  *Acceptance:* Users see a clear non-technical message; normal UI is gated until recovery.  
  *Status:* ✅ Complete — `DegradedModeBanner` gates entire UI based on backend readiness state.
- **Conditional Dependency Polling:**  
  *Acceptance:* Startup and recovery converge to ready state without restart loops.  
  *Status:* ✅ Complete — Startup and recovery polling implemented; runtime failure detection integrated via `with_txn` and `RealtimeBroker` error reporting.
- **Operational Logging:**  
  *Acceptance:* Logs are actionable and explain why readiness is blocked.  
  *Status:* ✅ Complete — Structured logging for state transitions and dependency failures implemented.

---
 
# Optional & Enhancement Track
 
---
 
### ✅ **1. Code Organization & Refactoring**
- **Refactor `game-room-client.tsx`:**  
  *Acceptance:* Component refactored; complexity reduced; maintainability improved.  
  *Status:* ✅ Complete — Reduced from 791 to 155 lines; state extracted into focused custom hooks; view logic separated.
 
---
 
### ✅ **2. Frontend Experience Enhancements**
- **React Query Adoption:**  
  *Acceptance:* React Query fully adopted; decision on optimistic updates documented.  
  *Status:* ✅ Complete — TanStack Query adopted; polling inefficiency addressed; centralized query keys; consistent error handling; optimistic updates deemed unnecessary.
- **Import Hygiene:**  
  *Acceptance:* Type-only imports enforced; consistent syntax across codebase.  
  *Status:* ✅ Complete — ESLint `consistent-type-imports` rule added; all violations fixed.
- **Tailwind CSS v3 to v4 Migration:**  
  *Acceptance:* Application runs on Tailwind v4 with styling preserved.  
  *Status:* ✅ Complete — Migrated to Tailwind v4; updated PostCSS plugin; preserved theme configuration.
 
---
 
### ✅ **3. Behavioral & Infrastructure Improvements**
- **Data & Auth Hygiene:**  
  *Acceptance:* Email normalization, validation, and username cleaning implemented and tested.  
  *Status:* ✅ Complete — `normalize_email()`, `validate_email()`, `derive_username()` implemented; redundant writes analysis completed.
- **PII-Safe Logging:**  
  *Acceptance:* Sensitive identifiers masked in all log output.  
  *Status:* ✅ Complete — `Redacted` wrapper masks emails, tokens, and `google_sub` values.
- **Error Code Catalog:**  
  *Acceptance:* All error codes use centralized enum; no ad-hoc strings.  
  *Status:* ✅ Complete — SCREAMING_SNAKE error codes centralized in `error_code.rs`.
 
---
 
### ✅ **4. Testing & Validation Enhancements**
- **Deterministic AI Simulation:**  
  *Acceptance:* Identical seeds yield identical results.  
  *Status:* ✅ Complete — Seed infrastructure implemented; AI decisions and game state reproducible across runs.
 
---
 
### ✅ **5. AI & Simulation Initiatives**
- **AI Profile Discovery & Registry Alignment:**  
  *Acceptance:* Contributors can register/discover AIs via a single authoritative source.  
  *Status:* ✅ Complete — Registry surfaces all AI profiles; onboarding docs updated.
- **Multi-Engine AI Implementation Drive:**  
  *Acceptance:* Each engine exposes at least one production-ready AI.  
  *Status:* ✅ Complete — Simulation and production engines expose documented AIs aligned with guide.
- **In-Memory AI Comparison Harness:**  
  *Acceptance:* Developers can pit AIs head-to-head and capture metrics.  
  *Status:* ✅ Complete — Benchmarking mode implemented for rapid experimentation.
 
---
 
### ✅ **6. Future Architecture Considerations**
- **State Management Library:**  
  *Acceptance:* Decision made; no external state management library needed.  
  *Status:* ✅ Complete — TanStack Query confirmed sufficient for server state and cache sync.
- **Component-Level Lazy Loading:**  
  *Acceptance:* Decision made; no lazy loading needed.  
  *Status:* ✅ Complete — Lazy loading deemed unnecessary given usage patterns.
 
---
 
### ✅ **7. Trace ID Logging Strategy Review**
- **Trace ID Logging Strategy Review:**  
  *Acceptance:* `trace_id` appears once per log line; consistent emission strategy.  
  *Status:* ✅ Complete — Span-only approach adopted for handlers; explicit logging retained where needed; WebSocket upgrade bridges HTTP `trace_id` to session lifecycle.
 
---
 
### 🟨 **8. CI Pipeline**
**Dependencies:** 4, 5, 6, 7, 9, 14, 15  
- **Local Pre-commit Hooks:**  
  *Acceptance:* Pre-commit hooks active.  
  *Status:* ✅ Complete — FE lint/format and BE clippy/rustfmt enforced locally.
- **Planned CI:**  
  *Acceptance:* CI green gate required for merges once introduced.  
  *Status:* ⬜ Deferred — Full GitHub Actions pipeline deferred as solo developer.
- **Security Scanning:**  
  *Acceptance:* Image scans run on CI and block merges on critical vulnerabilities.  
  *Status:* ⬜ Planned — Container vulnerability scanning task defined.
 
---
 
### 🟨 **9. Open Source Observability Stack**
**Dependencies:** 10  
- **Observability Stack:**  
  *Acceptance:* Metrics, logs, and traces visible in dashboards when enabled.  
  *Status:* ⬜ Deferred — Grafana, Tempo, Loki, Prometheus integration defined; implementation deferred until needed.
 
---
 
# Documentation Commitment (Ongoing)
- **README:**  
  *Acceptance:* New developers can onboard independently.  
  *Status:* Ongoing — Setup, reset flow, and architecture explanation kept current.
- **CONTRIBUTING:**  
  *Acceptance:* Architecture and layering guidelines documented.  
  *Status:* Ongoing — Module layout, extractor policy, `_test` guard, DTO policies maintained.
- **Inline Comments & JSDoc:**  
  *Acceptance:* Complex logic explained inline; public APIs documented.  
  *Status:* Ongoing — Documentation updated alongside code changes.
- **Environment Variable Documentation:**  
  *Acceptance:* Environment variables and security features documented.  
  *Status:* Ongoing — Comprehensive environment and security configuration documentation maintained.
- **Architecture Documentation:**  
  *Acceptance:* Architecture docs stay current with system changes.  
  *Status:* Ongoing — `docs/architecture-*.md` updated as system evolves.
 
---