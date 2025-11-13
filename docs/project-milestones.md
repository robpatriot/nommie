# 🗺️ Nommie — Milestone & Enhancement Roadmap

## Document Scope

Tracks delivery milestones, optional enhancements, and outstanding work items.
Use this alongside `../dev-roadmap.md` for UI sequencing and
`architecture-overview.md` for the technical baseline each milestone builds on.

This document outlines Nommie’s development path:
Core milestones first, then optional and enhancement tracks that can be implemented at any time.

---

## Core Milestones

### ✅ **1. Repository & Project Bootstrap**
**Dependencies:** none  
**Details:**
- Monorepo with `apps/frontend`, `apps/backend`, and `packages/`.
- Root `.env` is canonical; frontend `.env.local` mirrors only `NEXT_PUBLIC_*`.
- ESLint/Prettier configured for the frontend.
- Pre-commit hooks active.
- Scripts:
  - `backend:fmt` → `cargo fmt --manifest-path apps/backend/Cargo.toml --all`
  - `backend:clippy` → `cargo clippy --manifest-path apps/backend/Cargo.toml --all-targets --all-features -- -D warnings`
**Acceptance:** Hello-world frontend and backend build locally; lint and format hooks pass.

---

### ✅ **2. Docker-First Development Environment**
**Dependencies:** 1  
**Details:**
- Docker Compose with Postgres (roles, DBs, grants).
- Host-pnpm for speed; backend runs on host or in container.
**Acceptance:** `pnpm dev` starts frontend and backend; Postgres reachable; frontend communicates with backend.

---

### ✅ **3. Database Schema via Init SQL (Scaffolding Only)**
**Dependencies:** 2  
**Details:**
- Single `init.sql` is source of truth.
- Test harness applies schema to `_test` database at startup with guard.
**Acceptance:** Tests bootstrap schema cleanly; `_test` guard enforced.  
*(Actual entities defined in milestone 6.)*

---

### ✅ **4. Testing Harness & Policies**
**Dependencies:** 3  
**Details:**
- `pnpm test` runs unit, integration, and smoke tests.
- Actix in-process integration harness.
- First smoke test: `create → add AI → snapshot`.
**Acceptance:** Tests pass locally and in CI.

---

### ✅ **5. Error Shapes & Logging**
**Dependencies:** 4  
**Details:**
- Problem Details error format: `{ type, title, status, detail, code, trace_id }`.
- `code` uses SCREAMING_SNAKE convention.
- Middleware assigns a `trace_id` per request.
**Acceptance:** Consistent error responses; logs include `trace_id`.

---

### ✅ **6. Database Schema (Actual Entities)**
**Dependencies:** 3, 4  
**Details:**
- Entities defined in `init.sql`: `users`, `games`, `memberships`, `bids`, `plays`, `scores`.
- Enums for game and membership states.
- Foreign keys and indexes added.
- AI players represented in `users` table like humans.
**Acceptance:** Schema applies cleanly and aligns with game lifecycle.

---

### ✅ **7. User Authentication**
**Dependencies:** 6  
**Details:**
- Google OAuth for login and account creation.
- JWTs for frontend/backend authentication.
- Authentication extractor validates JWT and resolves current user.
**Acceptance:** Users authenticate via Google; JWT validation works end-to-end.

---

### ✅ **8. Transactional Tests & DB Access Pattern**
**Dependencies:** 4  
**Details:**
- Unified request-path DB access through `with_txn`.
- Rollback-by-default test policy.
- Nested `with_txn` behavior defined and tested.
**Acceptance:** All handlers use `with_txn`; no direct `state.db` usage; lint and tests clean.

---

### ✅ **9. Extractors**
**Dependencies:** 5, 6, 7  
**Details:**
- Implemented: `AuthToken`, `JwtClaims`, `CurrentUser`, `GameId`, `GameMembership`, and `ValidatedJson<T>`.
**Acceptance:** Handlers are thin; extractor tests pass; single DB hit for user and membership.

---

### 🟨 **10. Backend Domain Modules**
**Dependencies:** 7  
**Details:**
- Pure logic modules: `rules`, `bidding`, `tricks`, `scoring`, `state`.
- No SeaORM in domain modules.
**Progress:** ORM isolation and adapter layer complete; foundation ready for domain logic.  
**Acceptance:** `grep` shows no ORM usage in domain code.

---

### 🟨 **11. Frontend App Router Seed**
**Dependencies:** 5, 7  
**Details:**
- Next.js App Router + Turbopack.
- Login page working.
- Lobby and Game skeleton pages pending.
**Acceptance:** Users can sign in and access placeholder lobby/game views.

---

### 🟨 **12. Game Lifecycle (Happy Path)**
**Dependencies:** 9, 7, 10, 11  
**Details:**
- Complete flow: `create → join → ready → deal → bid → trump → tricks → scoring → next round`.
- Integration test covers minimal end-to-end loop.
**Progress:** Adapters, services, and transactional sentinel in place; game orchestration logic pending.  
**Acceptance:** A full happy-path game completes successfully.

---

### 🕓 **13. AI Orchestration**
**Dependencies:** 11  
**Details:**
- AI performs bidding and legal plays.
- Game advances automatically until human input is required.
**Progress:** Infrastructure and deterministic test harness ready; logic pending.  
**Acceptance:** Full AI-only games complete successfully.

---

### 🕓 **14. Validation, Edge Cases, and Property Tests**
**Dependencies:** 11  
**Details:**
- Invalid bids/plays return proper Problem Details.
- Property tests confirm trick/scoring invariants.
**Progress:** Core correctness and timestamp tests complete; property-based testing pending.  
**Acceptance:** Error cases handled consistently; all properties hold.

---

### ✅ **15. Frontend UX Pass (Round 1)**
**Dependencies:** 11, 13  
**Details:**
- Hand display, trick area, bidding UI, trump selector.
- Frontend shows Problem Details errors clearly.
**Acceptance:** Gameplay readable and intuitive.

---

### 🟨 **16. CI Pipeline**
**Dependencies:** 4, 5, 6, 7, 9, 14, 15  
**Details:**
- Local: pre-commit hooks with FE lint/format and BE clippy/rustfmt.
- Planned CI: GitHub Actions gates merges with lint, tests, and schema checks.
**Progress:** Local grep gates and lint/test guards complete; remote CI integration pending.  
**Acceptance:** CI green gate required for merges; schema re-applies cleanly.

---

### 🟨 **17. Documentation & Decision Log**
**Dependencies:** 11  
**Details:**
- README: setup and reset flow.
- CONTRIBUTING: module layout, extractor policy, `_test` guard.
- DECISIONS.md: locked technical decisions.
**Progress:** `.cursorrules` and roadmap current; README/CONTRIBUTING need refresh for layering and DTO policies.  
**Acceptance:** New developers can onboard independently.

---

### 🕓 **18. Observability & Stability**
**Dependencies:** 5, 11  
**Details:**
- Logs include `user_id` and `game_id` when relevant.
- Frontend displays `trace_id` on error surfaces.
- `/health` endpoint checks DB connectivity.
**Progress:** Trace IDs active; enrichment and health route pending.  
**Acceptance:** Logs actionable; trace ID visible end-to-end.

---

### 🕓 **19. Open Source Observability Stack**
**Dependencies:** 16, 10  
**Details:**
- Grafana, Tempo, Loki, and Prometheus in Docker Compose.
**Progress:** Docker baseline complete; observability stack not yet deployed.  
**Acceptance:** Metrics, logs, and traces integrated and viewable.

---

## Optional & Enhancement Track

Independent improvements that enhance robustness, performance, and developer experience.

---

### **1. Architecture & Reliability**
- **WebSockets / Server Push:** Replace polling with WebSockets or SSE.  
  *Acceptance:* Real-time updates replace polling.
- **Deployment Stub:** Minimal production-style environment including FE, BE, DB, and observability stubs.  
  *Acceptance:* Application runs in minimal production configuration.
- **Race-Safe `ensure_user`:** Handle concurrent insertions safely by re-fetching on unique violations.  
  *Acceptance:* No duplicate users under concurrency.

---

### **2. Behavioral & Infrastructure Improvements**
- **Data & Auth Hygiene:** Email normalization (trim, lowercase, Unicode NFKC), validation, username cleaning, skip redundant writes.  
- **PII-Safe Logging:** Mask or hash sensitive identifiers in logs.  
- **Error Code Catalog:** Centralize all SCREAMING_SNAKE error codes.  
- **Rate Limiting:** Apply `429 RATE_LIMITED` to authentication endpoints.  
- **Determinism Tools:** Introduce injectable clock, seeded RNG, and mock time for reproducible tests.  

*Progress:* Transactional harness and DTO structure already support deterministic time injection and data hygiene hooks.

---

### **3. Extractors, Testing, and Validation**
- **Extractor Unification:** Ensure all routes use `ValidatedJson<T>`, `AuthToken`, `CurrentUser`, `GameId`, and `GameMembership`.  
  *Acceptance:* Input validation consistent across all handlers.
- **Extended Property Tests:** Verify correctness for dealing, progression, scoring, bidding, and serialization invariants.  
  *Acceptance:* Invariants hold across generated games.  
- **Golden Snapshot Fixtures:** Canonical JSON snapshots for all game phases, shared between frontend and backend.  
  *Acceptance:* Schema or logic changes surface as test diffs.  
- **Deterministic AI Simulation:** Replay identical seeded games for regression testing.  
  *Acceptance:* Identical seeds yield identical results.

---

### **4. Developer Experience & Frontend Quality**
- **Import Hygiene & Lazy Loading:** Standardized import order, type-only imports, and dynamic loading for heavy libraries.  
  *Acceptance:* Consistent imports and improved build performance.  
- **Documentation Enhancements:** Add `DECISIONS.md`, `CONTRIBUTING.md`, and `_test` guard docs.  
  *Acceptance:* Onboarding and contribution processes are self-contained.  
- **Frontend Polish:** Continue refining UI clarity and responsiveness beyond Round 1.

---

### **5. Observability & Health**
- **Health Endpoint:** Add `/health` route reporting DB connectivity and version info.  
  *Acceptance:* Endpoint returns up/down status with trace context.  
- **Observability Stack:** Integrate Grafana, Tempo, Loki, and Prometheus for full observability.  
  *Acceptance:* Metrics, logs, and traces visible in dashboards.  
- **Trace Context Enrichment:** Logs always include `trace_id`, `user_id`, and `game_id`.
