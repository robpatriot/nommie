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
- **Monorepo Setup:** ✅ **Completed** — Monorepo with `apps/frontend`, `apps/backend`, and `packages/`. Root `.env` is canonical; frontend `.env.local` mirrors only `NEXT_PUBLIC_*`.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Hello-world frontend and backend build locally.
- **Linting & Formatting:** ✅ **Completed** — ESLint/Prettier configured for the frontend. Pre-commit hooks active. Scripts: `backend:fmt` → `cargo fmt --manifest-path apps/backend/Cargo.toml --all`; `backend:clippy` → `cargo clippy --manifest-path apps/backend/Cargo.toml --all-targets --all-features -- -D warnings`.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Lint and format hooks pass.

---

### ✅ **2. Docker-First Development Environment**
**Dependencies:** 1  
- **Docker Compose Setup:** ✅ **Completed** — Docker Compose with Postgres (roles, DBs, grants). Host-pnpm for speed; backend runs on host or in container.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ `pnpm start` starts frontend and backend; frontend communicates with backend.
- **Postgres TLS/SSL Support:** ✅ **Completed** — Postgres connections use TLS with `verify-full` default; shared Postgres TLS image with build-time certificate generation; separate volume for certificates.  
  *Status:* ✅ Complete. TLS-enabled Postgres configured; certificates managed via shared volume; backend supports TLS connections with verify-full validation.  
  *Acceptance:* ✅ Postgres reachable with TLS.

---

### ✅ **3. Database Schema via Init SQL (Scaffolding Only)**
**Dependencies:** 2  
- **Schema Management:** ✅ **Completed** — Single `init.sql` is source of truth. Test harness applies schema to `_test` database at startup with guard.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Tests bootstrap schema cleanly; `_test` guard enforced.  
*(Actual entities defined in milestone 6.)*

---

### ✅ **4. Testing Harness & Policies**
**Dependencies:** 3  
- **Test Infrastructure:** ✅ **Completed** — `pnpm test` runs unit, integration, and smoke tests. Actix in-process integration harness. First smoke test: `create → add AI → snapshot`.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Tests pass locally and in CI.

---

### ✅ **5. Error Shapes & Logging**
**Dependencies:** 4  
- **Problem Details Format:** ✅ **Completed** — Problem Details error format: `{ type, title, status, detail, code, trace_id }`. `code` uses SCREAMING_SNAKE convention. Middleware assigns a `trace_id` per request.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Consistent error responses; logs include `trace_id`.

---

### ✅ **6. Database Schema (Actual Entities)**
**Dependencies:** 3, 4  
- **Entity Definitions:** ✅ **Completed** — Entities defined in `init.sql`: `users`, `games`, `memberships`, `bids`, `plays`, `scores`. Enums for game and membership states. Foreign keys and indexes added. AI players represented in `users` table like humans.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Schema applies cleanly and aligns with game lifecycle.

---

### ✅ **7. User Authentication**
**Dependencies:** 6  
- **OAuth & JWT:** ✅ **Completed** — Google OAuth for login and account creation. JWTs for frontend/backend authentication. Authentication extractor validates JWT and resolves current user.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Users authenticate via Google; JWT validation works end-to-end.

---

### ✅ **8. Transactional Tests & DB Access Pattern**
**Dependencies:** 4  
- **Transaction Management:** ✅ **Completed** — Unified request-path DB access through `with_txn`. Rollback-by-default test policy. Nested `with_txn` behavior defined and tested.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ All handlers use `with_txn`; no direct `state.db` usage; lint and tests clean.
- **Determinism Tools:** ✅ **Completed** — Injectable clock, seeded RNG, and mock time for reproducible tests (transactional harness and DTO structure already support deterministic time injection).  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Tests are reproducible with deterministic time injection.

---

### ✅ **9. Extractors**
**Dependencies:** 5, 6, 7  
- **Extractor Implementation:** ✅ **Completed** — Implemented: `AuthToken`, `JwtClaims`, `CurrentUser`, `GameId`, `GameMembership`, and `ValidatedJson<T>`.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Handlers are thin; extractor tests pass; single DB hit for user and membership; input validation consistent across all handlers.
- **Extractor Unification:** ✅ **Completed** — All routes use `ValidatedJson<T>`, `AuthToken`, `CurrentUser`, `GameId`, and `GameMembership` consistently.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Input validation consistent across all handlers.

---

### ✅ **10. Backend Domain Modules**
**Dependencies:** 7  
- **Pure Domain Logic:** ✅ **Completed** — Pure logic modules: `rules`, `bidding`, `tricks`, `scoring`, `state`. No SeaORM in domain modules.  
  *Status:* ✅ Complete. All domain modules are ORM-free. `CurrentRoundInfo::load()` and `GameHistory::load()` moved to `repos::player_view`. `CurrentRoundInfo` now uses domain `Phase` enum instead of `DbGameState`.  
  *Acceptance:* ✅ `grep` shows no ORM usage in domain code.

---

### ✅ **11. Frontend App Router Seed**
**Dependencies:** 5, 7  
- **Next.js App Router:** ✅ **Completed** — Next.js App Router with server components/actions, guarded by backend JWT resolution. Authenticated layout with shared header, theme provider, and suspense-loading states. Lobby and Game routes backed by live data fetching (ETag-aware snapshot polling) and server mutations.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Users can authenticate and reach lobby/game views with real data and actions wired end-to-end.

---

### ✅ **12. Game Lifecycle (Happy Path)**
**Dependencies:** 9, 7, 10, 11  
- **Complete Game Flow:** ✅ **Completed** — Complete flow: `create → join → ready → deal → bid → trump → tricks → scoring → next round`. Integration test covers minimal end-to-end loop.  
  *Status:* ✅ Complete. `services::game_flow` exercises full round progression with scoring, and `tests/suites/services/game_flow_happy_paths.rs` verifies deal→bid→play→score transitions.  
  *Acceptance:* ✅ A full happy-path game completes successfully with deterministic tests guarding regressions.

---

### ✅ **13. AI Orchestration**
**Dependencies:** 11  
- **AI Automation:** ✅ **Completed** — AI performs bidding and legal plays. Game advances automatically until human input is required.  
  *Status:* ✅ Complete. `GameFlowService::process_game_state` drives automatic turns with retry logic, `round_cache` eliminates redundant reads, and per-instance AI overrides merge profile + game config.  
  *Acceptance:* ✅ Full AI-only games complete successfully; orchestration tests cover bidding, trump selection, trick play, and auto-start flows.

---

### ✅ **14. Validation, Edge Cases, and Property Tests**
**Dependencies:** 11  
- **Error Handling:** ✅ **Completed** — Invalid bids/plays return proper Problem Details.  
  *Status:* ✅ Complete. Service suites assert Problem Details codes for invalid bids/plays.  
  *Acceptance:* ✅ Error cases handled consistently.
- **Property Tests:** ✅ **Completed** — Property tests confirm trick/scoring invariants. Extended property tests verify correctness for dealing, progression, scoring, bidding, and serialization invariants (bidding, tricks, legality, consistency already covered).  
  *Status:* ✅ Complete. `domain/tests_props_*.rs` proptest suites lock in trick legality, scoring, and consistency invariants (with regression seeds tracked).  
  *Acceptance:* ✅ All properties hold across generated games; invariants verified for dealing, progression, scoring, bidding, and serialization.

---

### ✅ **15. Frontend UX Pass (Round 1)**
**Dependencies:** 11, 13  
- **Core Game UI:** ✅ **Completed** — Hand display, trick area, bidding UI, trump selector. Frontend shows Problem Details errors clearly.  
  *Status:* ✅ Complete. Core components implemented and functional. Ongoing UX refinements include phase-specific waiting messages in trick area (shows "Waiting for bidding to complete…" during bidding, "Waiting for trumps to be chosen…" during trump selection, and "Waiting for lead…" during trick play).  
  *Acceptance:* ✅ Gameplay readable and intuitive.

---

### ✅ **16. Frontend UX Pass (Round 2)**
**Dependencies:** 15  
- **Design v1 Implementation:** ✅ **Completed** — Apply the first-endorsed product design across Nommie (typography, spacing, components).  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Core screens match design reference.
- **Game Config vs Play UI Split:** ✅ **Completed** — Split the game experience into a configuration surface (seating, AI seats, options) and an in-game surface focused on play.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Users transition smoothly between config and play areas.
- **Last Trick UI:** ✅ **Completed** — Persist the most recent trick as a compact card row so play can continue immediately after the final card.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Previous trick reviewable.
- **User Options:** ✅ **Completed** — Add per-account settings (e.g., theme, gameplay preferences) surfaced via a profile/options view.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Account preferences persist.
- **Card Play Confirmation Toggle:** ✅ **Completed** — Provide a per-account option for confirming card plays before submission.  
  *Status:* ✅ Complete. Stage 1 UI roadmap items are complete—Design v1, the config/play split, Last Trick UI, user options, confirmation toggle, and polish/animation passes are live in production.  
  *Acceptance:* ✅ Card confirmation toggle works.

---

### ✅ **17. Mobile Design & UI Implementation**
**Dependencies:** 11, 15, 16  
- **Design System Parity:** ✅ **Completed** — Define a mobile-specific design kit (spacing, typography, colors, components) that mirrors the web experience while honoring native platform conventions and accessibility.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Navigation, theming, and interactions feel native.
- **Expo App Foundations:** ✅ **Completed** — Scaffold the `apps/mobile` Expo client with navigation (stack + modal flows), theming, and auth hand-off using the existing backend JWT flow.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Mobile users can authenticate.
- **End-to-End Screens:** ✅ **Completed** — Implement lobby list, game configuration, and in-game play surfaces (bidding, trump select, trick play, last-trick review) with responsive layouts, gestures, and haptics.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Mobile users can configure games and play full rounds with UX parity to the web client.
- **State & Sync:** ✅ **Completed** — Reuse shared types/API wrapper and support offline/foreground-resume states with snapshot hydration.  
  *Status:* ✅ Complete. Mobile UX parity delivered—Expo app foundations, theming/auth hand-off, core screens (lobby, config, play), and sync/resume flows are complete.  
  *Acceptance:* ✅ The app handles reconnects and snapshot refreshes gracefully.

---

### ✅ **18. Architecture & Reliability**
- **WebSockets / Server Push & Architecture:** ✅ **Completed** — Replace polling with WebSockets (or SSE) and decide on the long‑term realtime architecture and testing strategy. Add end-to-end WebSocket integration tests for game sessions (connect, initial snapshot, broadcasts, shutdown). Document the chosen realtime architecture (registry/broker split, Redis pub/sub fan-out) and how it is tested.  
  *Status:* ✅ Complete. WebSocket infrastructure implemented and deployed; polling replaced. Frontend uses `useGameSync` hook; backend publishes snapshots via Redis after mutations. End-to-end backend integration tests added covering connection (JWT auth, initial ack, initial snapshot), multi-client broadcast (all clients, game isolation), reconnect behavior, and shutdown (registry cleanup). Tests use in-memory registry for concurrency safety and transaction-per-test isolation. Architecture documented in `docs/websocket-design.md`.  
  *Acceptance:* ✅ WebSocket sync is the primary update mechanism for active game clients; the architecture and testing strategy are documented and enforced via automated tests.
- **Deployment Stub:** ✅ **Completed** — Minimal production-style environment including FE, BE, DB, and Redis.  
  *Status:* ✅ Complete. Application runs in `docker/local-prod` with TLS, Caddy reverse proxy, and all services containerized.  
  *Acceptance:* ✅ Application runs in a minimal production-style configuration.
- **Race-Safe `ensure_user`:** ✅ **Completed** — Handle concurrent insertions safely using non-aborting upserts (`ON CONFLICT DO NOTHING`) with follow-up SELECT.  
  *Status:* ✅ Complete. Concurrent OAuth logins for same email succeed without duplicate users/credentials or transaction aborts. `ensure_user_by_sub()` and `ensure_credentials_by_email()` adapters prevent transaction-aborting unique violations; cleanup logic ensures no orphan users on email ownership conflicts. Concurrency regression test proves correctness under parallel first-login scenarios.  
  *Acceptance:* ✅ No duplicate users/credentials under concurrency.

---

### ✅ **19. Observability & Stability**
**Dependencies:** 5, 11  
- **Trace Context Enrichment:** ✅ **Completed** — Logs always include `trace_id`, `user_id`, and `game_id` when relevant.  
  *Status:* ✅ Complete. `TraceSpan` middleware creates spans with `trace_id`, `user_id` (from JWT), and `game_id` (from path params). All logs within handlers automatically inherit these fields via instrumented spans.  
  *Acceptance:* ✅ Logs actionable; trace ID visible end-to-end.
- **Frontend Trace Display:** ✅ **Completed** — Frontend displays `trace_id` on error surfaces.  
  *Status:* ✅ Complete. Toast component displays `trace_id` for error toasts behind a collapsible "Show details" button. ErrorBoundary component now displays `trace_id` (along with status and code) for `BackendApiError` instances behind a collapsible "Show details" button, matching the Toast pattern.  
  *Acceptance:* ✅ Trace ID visible end-to-end.
- **Health Endpoint:** ✅ **Completed** — Add `/health` route reporting DB connectivity and version info.  
  *Status:* ✅ Complete. `/health` route implemented at `apps/backend/src/routes/health.rs`, reporting DB connectivity, app version, migration status, and timestamp. Response includes `X-Trace-Id` header automatically via middleware.  
  *Acceptance:* ✅ Endpoint returns up/down status with trace context.
- **Security Logging:** ✅ **Completed** — Structured security logging for authentication failures and rate limit hits with appropriate log levels and context.  
  *Status:* ✅ Complete. `login_failed()` and `rate_limit_hit()` functions log security events with `trace_id` and appropriate context. Auth failures logged in JWT validation; rate limits logged in structured logger middleware.  
  *Acceptance:* ✅ Security events logged with appropriate detail.

---

### ✅ **21. Internationalisation & Frontend Localisation**
**Dependencies:** 11, 15  
- **End-to-End Frontend Internationalisation:** ✅ **Completed** — Implement a code-driven i18n strategy using `next-intl`, with locale-aware request handling, message loading, and per-namespace translation files for all user-facing UI.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Frontend UI text is fully localized and driven by error codes and message keys rather than inline strings; adding or updating locales is a data change (messages) rather than a code change.
- **Error Localisation via Codes:** ✅ **Completed** — Backend exposes structured Problem Details with stable `code` values; frontend maps codes through a single source of truth (`i18n/errors.ts` + `errors.*` namespaces) to derive localized messages for toasts, error boundaries, and inline surfaces.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Error codes mapped to localized messages.
- **Coverage & Tooling:** ✅ **Completed** — All interactive frontend flows (lobby, game room, settings, actions, toasts) use translations instead of hard-coded strings; multiple locales (`en-GB`, `fr-FR`, `de-DE`, `es-ES`, `it-IT`) are kept in sync via i18n lint scripts (`i18n-check`, `i18n-unused`) wired into `pnpm lint`. Debug/log output remains English-only and is not localized.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ I18n consistency is enforced in the lint pipeline.
- **Error Code Enforcement:** ✅ **Completed** — Frontend i18n check script verifies that all `KNOWN_ERROR_CODES` have corresponding translation keys in all locale `errors.json` files, ensuring complete coverage. Frontend logs warnings when encountering unknown error codes not present in the i18n key registry.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Error code coverage is verified automatically.
- **Systematic Date/Number Formatting:** ✅ **Completed** — Locale-aware formatting for dates, times, and numbers using `Intl` APIs (`Intl.DateTimeFormat`, `Intl.NumberFormat`) implemented via centralized utilities (`utils/date-formatting.ts`, `utils/number-formatting.ts`). All user-facing dates, timestamps, durations, and numeric values (round numbers, player counts, hand sizes, performance metrics) are formatted according to the user's locale preferences.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Date and number formatting respects user locale preferences.

---

### ✅ **22. Email Allowlist & Access Control**
**Dependencies:** 7  
- **Email Allowlist:** ✅ **Completed** — Implement email allowlist for signup and login to restrict access to authorized email addresses.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Only emails on the allowlist can sign up or log in.
- **Backend Implementation:** ✅ **Completed** — Allowlist validation in authentication flow with configurable allowlist via environment variables.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Configuration is documented and testable.
- **Frontend Error Handling:** ✅ **Completed** — Frontend handles allowlist errors gracefully with sign-out flow when access is denied.  
  *Status:* ✅ Complete. Email allowlist fully implemented in backend and frontend; configuration documented; tests updated to avoid env var dependencies.  
  *Acceptance:* ✅ Denied access triggers appropriate error handling and sign-out.
- **StateBuilder Integration:** ✅ **Completed** — Allowlist ownership managed through StateBuilder for consistent configuration.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Configuration is consistent.
- **Documentation:** ✅ **Completed** — Email allowlist configuration documented with environment variable setup.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Configuration is documented.

---

### ✅ **23. Security Hardening**
**Dependencies:** 2, 7  
- **Docker Image Hardening:** ✅ **Completed** — Non-root users and pinned base images for backend and frontend containers.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Containers run as non-root.
- **Security Headers:** ✅ **Completed** — Content Security Policy (CSP), Permissions-Policy, and X-XSS-Protection headers implemented.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Security headers present.
- **Rate Limiting:** ✅ **Completed** — Rate limiting middleware with security-specific logging for rate limit hits.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Rate limiting functional.
- **CORS Configuration:** ✅ **Completed** — Tightened CORS configuration for backend API.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ CORS properly configured.
- **Environment Validation:** ✅ **Completed** — Startup validation for critical backend environment variables.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Environment validation in place.
- **Authentication Security:** ✅ **Completed** — JWT lifetime adjustments, NextAuth session lifetime shortened, NextAuth updated to address security vulnerabilities.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Authentication lifetimes adjusted.
- **Upload Limits:** ✅ **Completed** — Universal upload limits implemented.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Upload limits enforced.
- **Error Security:** ✅ **Completed** — Avoid leaking user existence in forbidden user errors.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ No information leakage in errors.
- **Connection Security:** ✅ **Completed** — Postgres credentials percent-encoded in connection URLs.  
  *Status:* ✅ Complete. Docker images hardened; security headers implemented; rate limiting active; CORS tightened; environment validation in place; authentication lifetimes adjusted; upload limits enforced; error messages sanitized.  
  *Acceptance:* ✅ All security hardening measures implemented and tested.

---

### 🟨 **24. Documentation Maintenance (Ongoing)**
**Dependencies:** 11  
**Status:** Long-standing milestone — documentation is continuously maintained and updated as the project evolves.

- **README:** Setup and reset flow; architecture explanation (including layering and DTO policies).  
  *Status:* ✅ Current.  
  *Acceptance:* ✅ New developers can onboard independently.
- **CONTRIBUTING:** Module layout, extractor policy, `_test` guard, layering guidelines, and DTO policies.  
  *Status:* ✅ Current. README and CONTRIBUTING updated with layering and DTO policies.  
  *Acceptance:* ✅ Architecture is documented.
- **Inline Comments:** Add comments for complex logic (e.g., JWT refresh, domain algorithms) as code evolves.  
  *Status:* ✅ Ongoing. JSDoc and inline comments are added incrementally as new code is written.  
  *Acceptance:* ✅ Complex logic is explained inline.
- **JSDoc Documentation:** Add JSDoc for public APIs and complex functions as new features are added.  
  *Status:* ✅ Ongoing.  
  *Acceptance:* ✅ APIs have JSDoc comments.
- **Environment Variable Documentation:** Comprehensive documentation for all environment variables including security-related configuration.  
  *Status:* ✅ Current. Environment variable documentation improved.  
  *Acceptance:* ✅ Environment variables and security features are documented.
- **TLS Setup Documentation:** Documentation for Postgres TLS/SSL configuration and certificate management.  
  *Status:* ✅ Current. TLS setup documented.  
  *Acceptance:* ✅ TLS configuration documented.
- **Email Allowlist Documentation:** Configuration documentation for email allowlist feature.  
  *Status:* ✅ Current. Email allowlist configuration documented.  
  *Acceptance:* ✅ Email allowlist documented.
- **Architecture Documentation:** Keep architecture docs (`docs/architecture-*.md`) current with system changes.  
  *Status:* ✅ Current. `.cursorrules` and roadmap current.  
  *Acceptance:* ✅ Documentation stays current with codebase changes.

**Note:** This milestone is never "complete" — it represents an ongoing commitment to maintain documentation quality as the project grows. Documentation should be updated alongside code changes, not as a separate phase.

---

## Optional & Enhancement Track

Independent improvements that enhance robustness, performance, and developer experience.

---

### ✅ **1. Code Organization & Refactoring**
- **Refactor `game-room-client.tsx`:** ✅ **Completed** — The component has been refactored from 791 lines to 155 lines (80% reduction). State management extracted into focused custom hooks: `useGameRoomReadyState`, `useGameRoomActions`, `useGameRoomControls`, `useAiSeatManagement`, and `useSlowSyncIndicator`. View logic separated into `GameRoomView` component.  
  *Status:* ✅ Complete.  
  *Acceptance:* ✅ Component is refactored with improved state management; complexity is reduced; maintainability is improved.

---

### ✅ **2. Frontend Experience Enhancements**
- **React Query Adoption:** ✅ **Completed** — React Query (TanStack Query) has been adopted for client data fetching.
  - ✅ **Completed:** Polling inefficiency addressed (ETag-based caching and not_modified handling).
  - ✅ **Completed:** Request deduplication works automatically via React Query.
  - ✅ **Completed:** Centralized query key factory implemented for cache management.
  - ✅ **Completed:** Consistent error handling across query hooks.
  - ✅ **Completed:** Caching and state synchronization improved with proper invalidation strategies.
  - ✅ **Decision made:** Optimistic updates determined to be unnecessary — WebSocket updates provide real-time state synchronization, making optimistic updates redundant. Decision aligns with track 6 architecture review.
  *Status:* ✅ Complete. All React Query enhancements implemented; optimistic updates decided against.  
  *Acceptance:* ✅ React Query fully adopted with all planned enhancements; decision on optimistic updates documented.
- **Import Hygiene:** ✅ **Completed** — Type-only imports are now enforced via ESLint rule `@typescript-eslint/consistent-type-imports`. All type-only imports use `import type` syntax, improving tree-shaking and build performance. Lazy loading removed from scope (not needed per track 6 decision).  
  *Status:* ✅ Complete. ESLint rule added and all violations auto-fixed. All tests pass.  
  *Acceptance:* ✅ Type-only imports are enforced; consistent import syntax across codebase.
- **Tailwind CSS v3 to v4 Migration:** ✅ **Completed** — Migrated from Tailwind CSS v3.4.19 to v4.0.6. Replaced PostCSS plugin with `@tailwindcss/postcss`, removed autoprefixer (now handled automatically), migrated CSS imports to `@import "tailwindcss"` with `@config` directive, and added preflight overrides for button cursor and dialog margins. Theme configuration preserved using CSS-first approach.  
  *Status:* ✅ Complete. Production build passes; all styling preserved.  
  *Acceptance:* ✅ Application successfully runs on Tailwind v4 with all styling preserved; configuration updated; breaking changes addressed.

---

### ✅ **3. Behavioral & Infrastructure Improvements**
- **Data & Auth Hygiene:** ✅ **Completed** — Email normalization (trim, lowercase, Unicode NFKC) implemented in `normalize_email()`; email validation implemented in `validate_email()`; username cleaning/derivation implemented in `derive_username()`. Skip redundant writes determined to be not needed — analysis shows all `update_game` calls either have actual field changes or intentionally need `lock_version` increments for WebSocket broadcasts. ETag-based caching handles read optimization (304 Not Modified).  
  *Status:* ✅ Complete. Email normalization, validation, and username cleaning are production-ready. Skip redundant writes not needed per codebase analysis.  
  *Acceptance:* ✅ Email normalization, validation, and username cleaning implemented and tested.
- **PII-Safe Logging:** ✅ **Completed** — Comprehensive PII redaction implemented in `apps/backend/src/logging/pii.rs`. `Redacted` wrapper type automatically masks emails (keeps first char, masks rest), base64/hex tokens, and google_sub values. Used in security logging (`login_failed`, `rate_limit_hit`) and throughout user service.  
  *Status:* ✅ Complete. All sensitive identifiers are masked/hashed in logs.  
  *Acceptance:* ✅ Sensitive identifiers (emails, tokens, google_sub) are masked in all log output.
- **Error Code Catalog:** ✅ **Completed** — All SCREAMING_SNAKE error codes centralized in `apps/backend/src/errors/error_code.rs` as a type-safe enum. Prevents ad-hoc error code strings. All codes documented and organized by category (Auth, Validation, Conflicts, System, etc.).  
  *Status:* ✅ Complete. Error codes are centralized, type-safe, and well-documented.  
  *Acceptance:* ✅ All error codes use the centralized enum; no ad-hoc error code strings.
- ~~**Rate Limiting:** Apply `429 RATE_LIMITED` to authentication endpoints.~~ ✅ **Completed:** Rate limiting middleware implemented with security-specific logging (see Milestone 23).

---

### ✅ **4. Testing & Validation Enhancements**
- **Deterministic AI Simulation:** ✅ **Completed** — Replay identical seeded games for regression testing.  
  *Status:* ✅ Complete. Seed infrastructure implemented (`rng_seed` field, seed derivation utilities). Tests verify identical seeds produce identical results for AI decisions (bidding, playing, trump selection), game state, and memory degradation. `test_seed()` utility provides deterministic seed generation from test names.  
  *Acceptance:* ✅ Identical seeds yield identical results.

---

### **5. AI & Simulation Initiatives**
- **AI Profile Discovery & Registry Alignment:** Audit current AI profile usage, enable discovery, and either sync profiles into the existing registry or replace the registry with profile-driven loading.  
  *Acceptance:* Contributors can register/discover AIs via a single authoritative source with clear onboarding steps.
- **Multi-Engine AI Implementation Drive:** Coordinate all simulation/production engines to deliver best-possible AI implementations aligned with the AI Player guide.  
  *Acceptance:* Each engine exposes at least one production-ready AI with documented characteristics.
- **In-Memory AI Comparison Harness:** Extend the in-memory engine with a lightweight benchmarking mode focused on head-to-head performance (minimal correctness checks).  
  *Acceptance:* Developers can pit AIs against each other rapidly and capture comparative metrics.

---

### ✅ **6. Future Architecture Considerations**
- **State Management Library:** ✅ **Decision made** — TanStack Query is sufficient for state management. Server state is managed via TanStack Query cache (single source of truth), WebSocket updates write directly to the cache, and local UI state is minimal and well-scoped via custom hooks. No need for Redux/Zustand.  
  *Status:* ✅ Complete. Architecture review confirms TanStack Query handles all state management needs without additional complexity.  
  *Acceptance:* ✅ Decision made; no external state management library needed.
- **Component-Level Lazy Loading:** ✅ **Decision made** — Lazy loading is not needed. Since 99% of user time and functionality is spent on the game page, the game room should be in the initial bundle for optimal performance. Lazy loading would add unnecessary delay to the primary user journey (lobby → game). The current bundle size is acceptable for the use case.  
  *Status:* ✅ Complete. Architecture review confirms lazy loading would not provide value given usage patterns.  
  *Acceptance:* ✅ Decision made; no lazy loading needed.

---

### ✅ **7. Trace ID Logging Strategy Review**
- **Trace ID Logging Strategy Review:** ✅ **Completed** — Decide on a single source of truth for `trace_id` emission (span-only vs. event field vs. conditional) so console and aggregated logs stay consistent without duplicate IDs.  
  *Status:* ✅ Complete. Implemented span-only approach for handler code (removed explicit `trace_id` from error.rs, db_errors.rs, validated_json.rs). Kept explicit `trace_id` for code outside spans (StructuredLogger middleware, security logging). Added ephemeral `trace_id` logging in WebSocket upgrade to bridge HTTP request `trace_id` to WebSocket `session_id` for end-to-end traceability.  
  *Acceptance:* ✅ `trace_id` appears once per log line; handler logs inherit from spans; request completion and security logs use explicit fields; WebSocket upgrade logs bridge HTTP `trace_id` to session lifecycle via `session_id`.

---

### 🟨 **8. CI Pipeline**
**Dependencies:** 4, 5, 6, 7, 9, 14, 15  
- **Local Pre-commit Hooks:** ✅ **Completed** — Pre-commit hooks with FE lint/format and BE clippy/rustfmt.  
  *Status:* ✅ Complete. Local grep gates and lint/test guards complete.  
  *Acceptance:* ✅ Pre-commit hooks active.
- **Planned CI:** GitHub Actions gates merges with lint, tests, and schema checks.  
  *Status:* Deferred. As a solo developer, local lint/test + pre-commit hooks are sufficient for now. Full CI will be added if/when collaboration increases or automated deploys make it clearly worthwhile.  
  *Acceptance:* CI green gate required for merges; schema re-applies cleanly (once CI is introduced).
- **Security Scanning:** Automated container image vulnerability scanning (e.g., Trivy, Snyk) for backend and frontend images.  
  *Status:* Container vulnerability scanning task defined.  
  *Acceptance:* Image scans run on CI and block merges on critical vulnerabilities.

---

### 🟨 **9. Open Source Observability Stack**
**Dependencies:** 10  
- **Observability Stack:** Integrate Grafana, Tempo, Loki, and Prometheus in Docker Compose for full observability.  
  *Status:* Deferred until needed. Docker baseline complete; observability stack implementation deferred. Can be implemented with opt-in approach using `OBSERVABILITY_ENABLED` environment variable (single switch for both dev and docker contexts). Implementation estimated at 6-10 hours with zero overhead when disabled.  
  *Acceptance:* Metrics, logs, and traces visible in dashboards when enabled.
