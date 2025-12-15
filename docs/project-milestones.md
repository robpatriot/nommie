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
- **Postgres TLS/SSL Support:** Postgres connections use TLS with `verify-full` default; shared Postgres TLS image with build-time certificate generation; separate volume for certificates.
**Progress:** TLS-enabled Postgres configured; certificates managed via shared volume; backend supports TLS connections with verify-full validation.  
**Acceptance:** `pnpm start` starts frontend and backend; Postgres reachable with TLS; frontend communicates with backend.

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
- **Determinism Tools:** Injectable clock, seeded RNG, and mock time for reproducible tests (transactional harness and DTO structure already support deterministic time injection).
**Acceptance:** All handlers use `with_txn`; no direct `state.db` usage; lint and tests clean; tests are reproducible with deterministic time injection.

---

### ✅ **9. Extractors**
**Dependencies:** 5, 6, 7  
**Details:**
- Implemented: `AuthToken`, `JwtClaims`, `CurrentUser`, `GameId`, `GameMembership`, and `ValidatedJson<T>`.
- **Extractor Unification:** Ensure all routes use `ValidatedJson<T>`, `AuthToken`, `CurrentUser`, `GameId`, and `GameMembership` consistently.
**Acceptance:** Handlers are thin; extractor tests pass; single DB hit for user and membership; input validation consistent across all handlers.

---

### ✅ **10. Backend Domain Modules**
**Dependencies:** 7  
**Details:**
- Pure logic modules: `rules`, `bidding`, `tricks`, `scoring`, `state`.
- No SeaORM in domain modules.
**Progress:** All domain modules are ORM-free. `CurrentRoundInfo::load()` and `GameHistory::load()` moved to `repos::player_view`. `CurrentRoundInfo` now uses domain `Phase` enum instead of `DbGameState`.  
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
- **Extended Property Tests:** Verify correctness for dealing, progression, scoring, bidding, and serialization invariants (bidding, tricks, legality, consistency already covered).
**Progress:** Service suites assert Problem Details codes for invalid bids/plays, while `domain/tests_props_*.rs` proptest suites lock in trick legality, scoring, and consistency invariants (with regression seeds tracked).  
**Acceptance:** Error cases handled consistently; all properties hold across generated games; invariants verified for dealing, progression, scoring, bidding, and serialization.

---

### ✅ **15. Frontend UX Pass (Round 1)**
**Dependencies:** 11, 13  
**Details:**
- Hand display, trick area, bidding UI, trump selector.
- Frontend shows Problem Details errors clearly.
**Acceptance:** Gameplay readable and intuitive.

---

### ✅ **16. Frontend UX Pass (Round 2)**
**Dependencies:** 15  
**Details:**
- **Implement Design v1:** Apply the first-endorsed product design across Nommie (typography, spacing, components).  
- **Separate Game Config vs Play UI:** Split the game experience into a configuration surface (seating, AI seats, options) and an in-game surface focused on play.  
- **Last Trick UI:** Persist the most recent trick as a compact card row so play can continue immediately after the final card.  
- **User Options:** Add per-account settings (e.g., theme, gameplay preferences) surfaced via a profile/options view.  
- **Card Play Confirmation Toggle:** Provide a per-account option for confirming card plays before submission.  
**Progress:** Stage 1 UI roadmap items are complete—Design v1, the config/play split, Last Trick UI, user options, confirmation toggle, and polish/animation passes are live in production.  
**Acceptance:** Core screens match design reference; users transition smoothly between config and play areas; previous trick reviewable; account preferences persist; card confirmation toggle works.

---

### 🟨 **17. Mobile Design & UI Implementation**
**Dependencies:** 11, 15, 16  
**Details:**
- **Design System Parity:** Define a mobile-specific design kit (spacing, typography, colors, components) that mirrors the web experience while honoring native platform conventions and accessibility.  
- **Expo App Foundations:** Scaffold the `apps/mobile` Expo client with navigation (stack + modal flows), theming, and auth hand-off using the existing backend JWT flow.  
- **End-to-End Screens:** Implement lobby list, game configuration, and in-game play surfaces (bidding, trump select, trick play, last-trick review) with responsive layouts, gestures, and haptics.  
- **State & Sync:** Reuse shared types/API wrapper, add optimistic interactions for bid/play actions, and support offline/foreground-resume states with snapshot hydration.  
**Acceptance:** Mobile users can authenticate, configure games, and play full rounds with UX parity to the web client; navigation, theming, and interactions feel native; the app handles reconnects and snapshot refreshes gracefully.

---

### 🟨 **18. Architecture & Reliability**
**Details:**
- **WebSockets / Server Push & Architecture:** Replace polling with WebSockets or SSE and decide on the long‑term WebSocket architecture and testing strategy.  
  - Add end-to-end WebSocket integration tests for game sessions (connect, snapshots, broadcasts, shutdown).  
  - Decide whether to split `ws` concerns into reusable realtime infrastructure (e.g., broker/registries) vs. binary-only handlers (e.g., route wiring), and document the rationale.  
  *Acceptance:* Real-time updates replace polling; WebSocket architecture and testing strategy are documented and enforced via automated tests.
- **Deployment Stub:** Minimal production-style environment including FE, BE, DB, and observability stubs.  
  *Acceptance:* Application runs in minimal production configuration.
- **Race-Safe `ensure_user`:** Handle concurrent insertions safely by re-fetching on unique violations.  
  *Acceptance:* No duplicate users under concurrency.

---

### 🟨 **19. CI Pipeline**
**Dependencies:** 4, 5, 6, 7, 9, 14, 15  
**Details:**
- Local: pre-commit hooks with FE lint/format and BE clippy/rustfmt.
- Planned CI: GitHub Actions gates merges with lint, tests, and schema checks.
- Security: Automated container image vulnerability scanning (e.g., Trivy, Snyk) for backend and frontend images.
**Progress:** Local grep gates and lint/test guards complete; container vulnerability scanning task defined; remote CI integration pending.  
**Acceptance:** CI green gate required for merges; schema re-applies cleanly; image scans run on CI and block merges on critical vulnerabilities.

---

### 🕓 **21. Observability & Stability**
**Dependencies:** 5, 11  
**Details:**
- **Trace Context Enrichment:** Logs always include `trace_id`, `user_id`, and `game_id` when relevant.
- **Frontend Trace Display:** Frontend displays `trace_id` on error surfaces.
- **Health Endpoint:** Add `/health` route reporting DB connectivity and version info.
- **Security Logging:** Structured security logging for authentication failures and rate limit hits with appropriate log levels and context.
**Progress:** Trace IDs active; structured security logging implemented for auth failures and rate limits; enrichment and health route pending.  
**Acceptance:** Logs actionable; trace ID visible end-to-end; security events logged with appropriate detail; endpoint returns up/down status with trace context.

---

### 🕓 **22. Open Source Observability Stack**
**Dependencies:** 19, 10  
**Details:**
- **Observability Stack:** Integrate Grafana, Tempo, Loki, and Prometheus in Docker Compose for full observability.
**Progress:** Docker baseline complete; observability stack not yet deployed.  
**Acceptance:** Metrics, logs, and traces visible in dashboards.

---

### **23. Accessibility (a11y)**
**Dependencies:** 11, 15  
**Details:**
- **Keyboard-First Play:** Full keyboard navigation and control for all game interactions (card selection, bidding, trump selection, card play).  
  - Lobby: Tab navigation through toolbar → header → game rows; Arrow Up/Down navigate rows; Enter joins; `r` refresh; `c` create; `f` focus search.
  - Game Room: `?` shortcuts menu; `g l` Lobby; `s` Sidebar; Hand (Arrows/Home/End/Enter/Esc); Bidding (number keys/Up/Down/Enter/Esc); Ready `y`; Start `Shift+S` (host confirm).
- **Focus Management:** Visible focus indicators, logical tab order, focus trapping in dialogs, focus restoration after actions.
- **ARIA Labels & Semantics:** 
  - All interactive elements have descriptive `aria-label` attributes (e.g., "Seven of Hearts, legal" for cards).
  - Semantic HTML (`<table>` for lobby, proper heading hierarchy, live regions for phase/turn announcements).
  - Contextual aria-labels that describe button state (pending, disabled, selected).
- **Color Contrast:** Minimum 4.5:1 contrast ratio for all text and interactive elements.
- **Motion Preferences:** Honor `prefers-reduced-motion` for animations (150–200ms ease-out for play/trick-win animations).
- **Screen Reader Support:** Live regions for phase/turn changes, proper announcements for game state updates.
- **Enhanced Accessibility:** Continue improving beyond current aria-label implementation (form inputs, complex interactions, error states).
**Acceptance:** Fully keyboard-operable gameplay; a11y checks pass basic audit (WCAG 2.1 Level AA); screen reader testing confirms usability; motion preferences respected.

---

### **24. Internationalisation Foundations**
**Dependencies:** 11, 15  
**Details:**
- **Internationalisation Foundations:** Define and implement the initial i18n strategy (framework choice, locale loading, placeholder translations) with scope finalised before execution.  
**Acceptance:** Core tooling and process for multiple locales is in place, even if only one language ships initially.

---

### ✅ **25. Email Allowlist & Access Control**
**Dependencies:** 7  
**Details:**
- **Email Allowlist:** Implement email allowlist for signup and login to restrict access to authorized email addresses.
- **Backend Implementation:** Allowlist validation in authentication flow with configurable allowlist via environment variables.
- **Frontend Error Handling:** Frontend handles allowlist errors gracefully with sign-out flow when access is denied.
- **StateBuilder Integration:** Allowlist ownership managed through StateBuilder for consistent configuration.
- **Documentation:** Email allowlist configuration documented with environment variable setup.
**Progress:** Email allowlist fully implemented in backend and frontend; configuration documented; tests updated to avoid env var dependencies.  
**Acceptance:** Only emails on the allowlist can sign up or log in; denied access triggers appropriate error handling and sign-out; configuration is documented and testable.

---

### ✅ **26. Security Hardening**
**Dependencies:** 2, 7  
**Details:**
- **Docker Image Hardening:** Non-root users and pinned base images for backend and frontend containers.
- **Security Headers:** Content Security Policy (CSP), Permissions-Policy, and X-XSS-Protection headers implemented.
- **Rate Limiting:** Rate limiting middleware with security-specific logging for rate limit hits.
- **CORS Configuration:** Tightened CORS configuration for backend API.
- **Environment Validation:** Startup validation for critical backend environment variables.
- **Authentication Security:** JWT lifetime adjustments, NextAuth session lifetime shortened, NextAuth updated to address security vulnerabilities.
- **Upload Limits:** Universal upload limits implemented.
- **Error Security:** Avoid leaking user existence in forbidden user errors.
- **Connection Security:** Postgres credentials percent-encoded in connection URLs.
**Progress:** Docker images hardened; security headers implemented; rate limiting active; CORS tightened; environment validation in place; authentication lifetimes adjusted; upload limits enforced; error messages sanitized.  
**Acceptance:** All security hardening measures implemented and tested; security headers present; rate limiting functional; no information leakage in errors; containers run as non-root.

---

### 🟨 **27. Documentation Maintenance (Ongoing)**
**Dependencies:** 11  
**Status:** Long-standing milestone — documentation is continuously maintained and updated as the project evolves.

**Details:**
- **README:** Setup and reset flow; architecture explanation (including layering and DTO policies).
- **CONTRIBUTING:** Module layout, extractor policy, `_test` guard, layering guidelines, and DTO policies.
- **Inline Comments:** Add comments for complex logic (e.g., JWT refresh, domain algorithms) as code evolves.
- **JSDoc Documentation:** Add JSDoc for public APIs and complex functions as new features are added.
- **Environment Variable Documentation:** Comprehensive documentation for all environment variables including security-related configuration.
- **TLS Setup Documentation:** Documentation for Postgres TLS/SSL configuration and certificate management.
- **Email Allowlist Documentation:** Configuration documentation for email allowlist feature.
- **Architecture Documentation:** Keep architecture docs (`docs/architecture-*.md`) current with system changes.

**Progress:** `.cursorrules` and roadmap current; environment variable documentation improved; TLS setup documented; email allowlist configuration documented; README and CONTRIBUTING updated with layering and DTO policies. JSDoc and inline comments are added incrementally as new code is written.  
**Acceptance:** New developers can onboard independently; architecture is documented; documentation stays current with codebase changes; complex logic is explained inline; APIs have JSDoc comments; environment variables and security features are documented.

**Note:** This milestone is never "complete" — it represents an ongoing commitment to maintain documentation quality as the project grows. Documentation should be updated alongside code changes, not as a separate phase.

---

## Optional & Enhancement Track

Independent improvements that enhance robustness, performance, and developer experience.

---

### **1. Code Organization & Refactoring**
- **Refactor `game-room-client.tsx`:** The component is 791 lines with complex state management that could benefit from a reducer pattern.  
  *Acceptance:* Component is refactored with improved state management; complexity is reduced; maintainability is improved.

---

### **2. Frontend Experience Enhancements**
- **React Query Adoption:** Introduce React Query for client data fetching (lobby lists, game snapshots) while keeping room for future caching/state patterns.
  - Addresses polling inefficiency (requests made even when nothing changed).
  - Provides request deduplication automatically.
  - Enables optimistic updates for game actions (bid, play, etc.).
  - Improves caching and state synchronization.
  - **Offline Detection and Retry Logic:** React Query provides automatic retry logic with configurable retry attempts and exponential backoff. Combined with network status detection, this enables graceful handling of offline scenarios and automatic retry when connectivity is restored.
  *Acceptance:* Critical frontend requests flow through React Query with documented fetch policies; polling is optimized; request deduplication works; optimistic updates are implemented where appropriate; application gracefully handles offline scenarios and retries failed requests when connectivity is restored.
- **Import Hygiene & Lazy Loading:** Standardized import order, type-only imports, and dynamic loading for heavy libraries.  
  *Acceptance:* Consistent imports and improved build performance.
- **Tailwind CSS v3 to v4 Migration:** Migrate from Tailwind CSS v3 to v4, updating configuration, utilities, and any breaking changes.  
  *Acceptance:* Application successfully runs on Tailwind v4 with all styling preserved; configuration updated; breaking changes addressed.
- **Frontend Polish:** Continue refining UI clarity and responsiveness beyond Round 1.

---

### **3. Behavioral & Infrastructure Improvements**
- **Data & Auth Hygiene:** Email normalization (trim, lowercase, Unicode NFKC), validation, username cleaning, skip redundant writes.  
- **PII-Safe Logging:** Mask or hash sensitive identifiers in logs.  
- **Error Code Catalog:** Centralize all SCREAMING_SNAKE error codes.  
- ~~**Rate Limiting:** Apply `429 RATE_LIMITED` to authentication endpoints.~~ ✅ **Completed:** Rate limiting middleware implemented with security-specific logging (see Milestone 26).

*Progress:* Transactional harness and DTO structure already support deterministic time injection and data hygiene hooks. Rate limiting implemented and active.

---

### **4. Testing & Validation Enhancements**
- **Golden Snapshot Fixtures:** Canonical JSON snapshots for all game phases, shared between frontend and backend.  
  *Acceptance:* Schema or logic changes surface as test diffs.  
- **Deterministic AI Simulation:** Replay identical seeded games for regression testing.  
  *Acceptance:* Identical seeds yield identical results.

---

### **5. AI & Simulation Initiatives**
- **AI Profile Discovery & Registry Alignment:** Audit current AI profile usage, enable discovery, and either sync profiles into the existing registry or replace the registry with profile-driven loading.  
  *Acceptance:* Contributors can register/discover AIs via a single authoritative source with clear onboarding steps.
- **Multi-Engine AI Implementation Drive:** Coordinate all simulation/production engines to deliver best-possible AI implementations aligned with the AI Player guide.  
  *Acceptance:* Each engine exposes at least one production-ready AI with documented characteristics.
- **In-Memory AI Comparison Harness:** Extend the in-memory engine with a lightweight benchmarking mode focused on head-to-head performance (minimal correctness checks).  
  *Acceptance:* Developers can pit AIs against each other rapidly and capture comparative metrics.

---

### **6. Future Architecture Considerations**
- **State Management Library:** Consider using a state management library (Redux/Zustand) for complex game state if needed.  
  *Acceptance:* Decision made on whether external state management is needed; if adopted, implementation is complete.
- **Component-Level Lazy Loading:** Add component-level lazy loading as an optimization for heavy components.  
  *Acceptance:* Heavy components are lazy-loaded; bundle size and initial load time are improved.

---

### **7. Trace ID Logging Strategy Review**
- **Trace ID Logging Strategy Review:** Decide on a single source of truth for `trace_id` emission (span-only vs. event field vs. conditional) so console and aggregated logs stay consistent without duplicate IDs.  
  *Acceptance:* Preferred logging strategy is chosen, documented, and implemented across middleware/tests.

---

### 🟨 **8. PATCH Endpoints with ETag Support**
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

