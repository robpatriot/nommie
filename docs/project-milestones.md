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
**Progress:** Core domain modules shipped, but `domain::player_view` still depends on `sea_orm::ConnectionTrait`; data-loading split into adapters remains TODO.  
**Acceptance:** `grep` shows no ORM usage in domain code.

---

### ✅ **11. Frontend App Router Seed**
**Dependencies:** 5, 7  
**Details:**
- Next.js App Router with server components/actions, guarded by backend JWT resolution.
- Authenticated layout with shared header, theme provider, and suspense-loading states.
- Lobby and Game routes backed by live data fetching (ETag-aware snapshot polling) and server mutations.
**Acceptance:** Users can authenticate and reach lobby/game views with real data and actions wired end-to-end.

---

### ✅ **12. Game Lifecycle (Happy Path)**
**Dependencies:** 9, 7, 10, 11  
**Details:**
- Complete flow: `create → join → ready → deal → bid → trump → tricks → scoring → next round`.
- Integration test covers minimal end-to-end loop.
**Progress:** `services::game_flow` exercises full round progression with scoring, and `tests/suites/services/game_flow_happy_paths.rs` verifies deal→bid→play→score transitions.  
**Acceptance:** A full happy-path game completes successfully with deterministic tests guarding regressions.

---

### ✅ **13. AI Orchestration**
**Dependencies:** 11  
**Details:**
- AI performs bidding and legal plays.
- Game advances automatically until human input is required.
**Progress:** `GameFlowService::process_game_state` drives automatic turns with retry logic, `round_cache` eliminates redundant reads, and per-instance AI overrides merge profile + game config.  
**Acceptance:** Full AI-only games complete successfully; orchestration tests cover bidding, trump selection, trick play, and auto-start flows.

---

### ✅ **14. Validation, Edge Cases, and Property Tests**
**Dependencies:** 11  
**Details:**
- Invalid bids/plays return proper Problem Details.
- Property tests confirm trick/scoring invariants.
**Progress:** Service suites assert Problem Details codes for invalid bids/plays, while `domain/tests_props_*.rs` proptest suites lock in trick legality, scoring, and consistency invariants (with regression seeds tracked).  
**Acceptance:** Error cases handled consistently; all properties hold across generated games.

---

### ✅ **15. Frontend UX Pass (Round 1)**
**Dependencies:** 11, 13  
**Details:**
- Hand display, trick area, bidding UI, trump selector.
- Frontend shows Problem Details errors clearly.
**Acceptance:** Gameplay readable and intuitive.

---

### 🟨 **16. Frontend UX Pass (Round 2)**
**Dependencies:** 15  
**Details:**
- **Implement Design v1:** Apply the first-endorsed product design across Nommie (typography, spacing, components).  
- **Separate Game Config vs Play UI:** Split the game experience into a configuration surface (seating, AI seats, options) and an in-game surface focused on play.  
- **Last Trick UI:** Persist the most recent trick as a compact card row so play can continue immediately after the final card.  
- **User Options:** Add per-account settings (e.g., theme, gameplay preferences) surfaced via a profile/options view.  
- **Card Play Confirmation Toggle:** Provide a per-account option for confirming card plays before submission.  
**Acceptance:** Core screens match design reference; users transition smoothly between config and play areas; previous trick reviewable; account preferences persist; card confirmation toggle works.

---

### 🟨 **17. PATCH Endpoints with ETag Support**
**Dependencies:** 5, 9, 12  
**Details:**
- Implement PATCH endpoints for **game configuration only** (not gameplay actions) with conditional request support via ETags.
- **Context:** Gameplay actions (bidding, playing cards, selecting trump) already use ETags via POST endpoints with `If-Match` headers (implemented in earlier milestones). This milestone adds PATCH endpoints for configuration changes.
- **Scope:** PATCH is used for game configuration changes such as:
  - Adding/removing AI seats
  - Updating game settings/options
  - Modifying game metadata (name, visibility, etc.)
- **Not in scope:** Gameplay actions (bidding, playing cards, selecting trump, marking ready) already use POST endpoints with ETag/If-Match support and remain unchanged.
- **ETag generation:** Generate strong ETags from resource version/state (e.g., using `lock_version` or content hash), consistent with existing POST gameplay endpoints.
- **If-Match header handling:** Require `If-Match` header for PATCH requests; validate ETag matches current resource version (same pattern as POST gameplay endpoints).
- **Error responses:**
  - `409 Conflict` with `OptimisticLock` error code when `If-Match` ETag is stale (resource modified concurrently).
  - `428 Precondition Required` when `If-Match` header is missing (required for PATCH operations, consistent with POST gameplay endpoints).
  - Include version information in error extensions for client retry logic.
- **Success behavior:** PATCH with matching ETag succeeds and increments resource version/ETag (returns new ETag in response header).
- **Test coverage:**
  - `patch_with_matching_if_match_succeeds_and_bumps_etag` - Valid PATCH updates resource and returns new ETag.
  - `patch_with_stale_if_match_returns_409_with_extensions` - Stale ETag returns 409 with version details in extensions.
  - `patch_missing_if_match_returns_428` - Missing `If-Match` header returns 428 Precondition Required (consistent with POST gameplay endpoints).
**Acceptance:** PATCH endpoints enforce ETag validation for configuration changes; gameplay actions continue using POST with existing ETag support; concurrent modification conflicts return structured 409 responses; all test cases pass.

---

### 🟨 **18. CI Pipeline**
**Dependencies:** 4, 5, 6, 7, 9, 14, 15  
**Details:**
- Local: pre-commit hooks with FE lint/format and BE clippy/rustfmt.
- Planned CI: GitHub Actions gates merges with lint, tests, and schema checks.
**Progress:** Local grep gates and lint/test guards complete; remote CI integration pending.  
**Acceptance:** CI green gate required for merges; schema re-applies cleanly.

---

### 🟨 **19. Documentation & Decision Log**
**Dependencies:** 11  
**Details:**
- README: setup and reset flow.
- CONTRIBUTING: module layout, extractor policy, `_test` guard.
- DECISIONS.md: locked technical decisions.
**Progress:** `.cursorrules` and roadmap current; README/CONTRIBUTING need refresh for layering and DTO policies.  
**Acceptance:** New developers can onboard independently.

---

### 🕓 **20. Observability & Stability**
**Dependencies:** 5, 11  
**Details:**
- Logs include `user_id` and `game_id` when relevant.
- Frontend displays `trace_id` on error surfaces.
- `/health` endpoint checks DB connectivity.
**Progress:** Trace IDs active; enrichment and health route pending.  
**Acceptance:** Logs actionable; trace ID visible end-to-end.

---

### 🕓 **21. Open Source Observability Stack**
**Dependencies:** 18, 10  
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

---

### **6. Frontend Experience Enhancements**
- **React Query Adoption:** Introduce React Query for client data fetching (lobby lists, game snapshots) while keeping room for future caching/state patterns.  
  *Acceptance:* Critical frontend requests flow through React Query with documented fetch policies.

---

### **7. AI & Simulation Initiatives**
- **AI Profile Discovery & Registry Alignment:** Audit current AI profile usage, enable discovery, and either sync profiles into the existing registry or replace the registry with profile-driven loading.  
  *Acceptance:* Contributors can register/discover AIs via a single authoritative source with clear onboarding steps.
- **Multi-Engine AI Implementation Drive:** Coordinate all simulation/production engines to deliver best-possible AI implementations aligned with the AI Player guide.  
  *Acceptance:* Each engine exposes at least one production-ready AI with documented characteristics.
- **In-Memory AI Comparison Harness:** Extend the in-memory engine with a lightweight benchmarking mode focused on head-to-head performance (minimal correctness checks).  
  *Acceptance:* Developers can pit AIs against each other rapidly and capture comparative metrics.

---

### **8. Internationalisation Foundations**
- **Internationalisation Foundations:** Define and implement the initial i18n strategy (framework choice, locale loading, placeholder translations) with scope finalised before execution.  
  *Acceptance:* Core tooling and process for multiple locales is in place, even if only one language ships initially.
