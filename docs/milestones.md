# 🗺️ Nommie — Ordered Milestones Roadmap

---

## ✅ A — Repo & Project Bootstrap
- **Dependencies:** none.
- **Details:**
  - Monorepo with `apps/frontend`, `apps/backend`, `packages/`.
  - Root `.env` is canonical; FE `.env.local` mirrors only `NEXT_PUBLIC_*`.
  - Root scripts:
    - `backend:fmt` → `cargo fmt --manifest-path apps/backend/Cargo.toml --all`
    - `backend:clippy` → `cargo clippy --manifest-path apps/backend/Cargo.toml --all-targets --all-features -- -D warnings`
  - ESLint/Prettier configured for FE.
  - Pre-commit hooks active.
- **Acceptance:** Hello-world FE/BE apps build locally; lint/format hooks pass.

---

## ✅ B — Docker-First Dev Environment
- **Dependencies:** A.
- **Details:**
  - Docker Compose with Postgres (roles, DBs, grants).
  - Host-pnpm for speed; backend runs host or container.
- **Acceptance:** `pnpm dev` starts FE+BE; Postgres reachable; FE talks to BE.

---

## ✅ C — Database Schema via Init SQL *(Scaffolding Only)*
- **Dependencies:** B.
- **Details:**
  - Single `init.sql` is source of truth.
  - Test harness applies schema to `_test` DB at startup with guard.
- **Acceptance:** Tests bootstrap schema cleanly; `_test` guard enforced.  
  *(Actual entities live in F.)*

---

## ✅ D — Testing Harness & Policies
- **Dependencies:** C.
- **Details:**
  - `pnpm test` runs unit + integration + smoke.
  - Actix in-process integration test harness.
  - First smoke test: `create → add AI → snapshot`.
- **Acceptance:** Tests green locally + CI.

---

## ✅ E — Error Shapes & Logging *(S → M)*
- **Dependencies:** D.
- **Details:**
  - Problem Details error shape: `{ type, title, status, detail, code, trace_id }`.
  - `code` in SCREAMING_SNAKE.
  - Per-request `trace_id` surfaced in logs.
- **Acceptance:** Consistent error responses; logs include `trace_id`.

---

## ✅ F — Database Schema (Actual Entities) *(M → L)*
- **Dependencies:** C, D.
- **Details:**
  - Entities in `init.sql`: `users`, `games`, `memberships`, `bids`, `plays`, `scores`.
  - Enums for `game_state`, `membership_role`, etc.
  - FKs + indexes; AI players represented in `users` like humans.
- **Acceptance:** Schema applies cleanly; coherent with lifecycle needs.

---

## ✅ G — User Authentication *(M → L)*
- **Dependencies:** F.
- **Details (Done):**
  - Google OAuth for account creation & login.
  - JWTs for FE/BE auth.
  - Auth extractor validates JWT and resolves current user.
- **Acceptance:** Users authenticate via Google; JWTs enforced; extractor loads current user.

---

## ✅ H — App Error & Trace ID via Web Boundary *(S → M)*
- **Dependencies:** D, E.
- **Details (Done):**
  - Removed `trace_id` from `AppError`.
  - Middleware issues per-request `trace_id`, stored in request context and set in `x-trace-id` header.
  - `ResponseError` reads `trace_id` from context when building Problem Details.
  - Removed `from_req`, `with_trace_id`, `ensure_trace_id`.
  - Updated tests assert header presence and parity with JSON `trace_id`.
- **Acceptance:** 
  - No `trace_id` in `AppError` or manual attachments.
  - Problem Details and `x-trace-id` header agree for all errors.
  - `pnpm be:lint` and `pnpm be:test` pass.

---

## ✅ I — Transactional Tests *(S → M)*
- **Dependencies:** D.
- **Details (Done):**
  - Unified request-path DB access through `with_txn`; removed direct `state.db` grabs.
  - Simplified `AppState.db` builder.
  - Defined + tested nested `with_txn` behavior.
  - Enforced rollback-by-default policy in tests.
- **Acceptance:** 
  - Request-path code consistently uses `with_txn`.
  - No-DB state returns `DB_UNAVAILABLE`.
  - Shared + nested txn behavior proven in tests.
  - CI green.

---

## ✅ J — Extractors *(M → L)*
- **Dependencies:** E, F, G.
- **Details (Done):**
  - Completed: `AuthToken`, `JwtClaims`, `CurrentUser`, `GameId`, `GameMembership`, `ValidatedJson<T>`.
- **Acceptance:** 
  - Handlers are thin.
  - Extractor tests pass.
  - Single DB hit for user+membership where possible.

---

## 🅚 K — Backend Domain Modules *(L)*
- **Dependencies:** G.
- **Details:**
  - Pure logic: `rules`, `bidding`, `tricks`, `scoring`, `state`.
  - No SeaORM in domain modules; orchestration sits above.
- **Acceptance:** `grep` shows no SeaORM in domain code.

---

## 🟨 L — Frontend App Router Seed *(M, partially done)*
- **Dependencies:** E, G.
- **Details:**
  - Next.js App Router + Turbopack.
  - Pages:
    - ✅ **Login** (NextAuth v5 wrapper for Google) — working.
    - ❌ **Lobby skeleton** — not yet implemented.
    - ❌ **Game skeleton** — not yet implemented.
- **Acceptance:** Can sign in, see lobby, and a placeholder game screen.

---

## 🅛 M — Game Lifecycle (Happy Path) *(L → XL)*
- **Dependencies:** J, G, K, L.
- **Details:**
  - End-to-end: `create → join → ready → deal → bid → trump → tricks → scoring → round advance`.
  - Integration test covers the minimal loop.
- **Acceptance:** Happy-path game completes.

---

## 🅜 N — AI Orchestration *(M → L)*
- **Dependencies:** L.
- **Details:**
  - Basic AI bidding + valid trick play.
  - Runs per poll cycle; AI auto-advances until human’s turn.
- **Acceptance:** Full game completes with AIs filling seats.

---

## 🅝 O — Validation, Edge Cases & Property Tests *(M)*
- **Dependencies:** L.
- **Details:**
  - Invalid bids/plays return proper Problem Details.
  - Property tests for trick/scoring invariants.
- **Acceptance:** Error paths validated; properties hold.

---

## 🅞 P — Frontend UX Pass (Round 1) *(M → L)*
- **Dependencies:** L, N.
- **Details:**
  - Hand display, trick area, bidding UI, trump selector.
  - FE surfaces Problem Details errors nicely.
- **Acceptance:** Gameplay is clear; errors understandable.

---

## 🟨 Q — CI Pipeline *(S, partially done)*
- **Dependencies:** D, E, F, G, J, O, P.
- **Completed (local):**
  - Robust pre-commit: FE ESLint/Prettier (staged-aware), BE clippy + rustfmt (staged write).
- **Remaining (for CI gate):**
  - GitHub Actions gating PRs/`main` with:
    - FE: ESLint, Prettier check, TS typecheck.
    - BE: clippy, `rustfmt --check`.
    - Tests with Postgres service; apply `init.sql` twice.
    - Caching (pnpm + Cargo).
- **Acceptance:** CI green gate required for merges; schema re-applies cleanly.

---

## 🅟 R — Documentation & Decision Log *(S)*
- **Dependencies:** L.
- **Details:**
  - README: setup + reset flow.
  - CONTRIBUTING: module layout, extractor policy, `_test` guard.
  - DECISIONS.md: locked decisions recorded.
- **Acceptance:** New devs onboard smoothly.

---

## 🅠 S — Observability & Stability *(S → M)*
- **Dependencies:** E, L.
- **Details:**
  - Logs include `user_id` + `game_id` where relevant.
  - FE shows `trace_id` on errors.
  - Health endpoint reports DB status.
- **Acceptance:** Logs actionable; trace id visible end-to-end.

---

## 🅡 T — Open Source Observability Stack *(M → L)*
- **Dependencies:** Q, K.
- **Details:**
  - Grafana + Tempo + Loki + Prometheus in Docker.
- **Acceptance:** Infra captures app metrics, logs, and traces.

---

# 🔄 Optional Track (anytime)

### 🅂 WebSockets *(M)*
- **Dependencies:** L.
- **Details:** Replace polling with push (Actix WS or SSE). Ensure AI orchestration fits push model.
- **Acceptance:** FE receives live state; polling removed.

### 🅄 Deployment Stub *(S → M)*
- **Dependencies:** B, R, S.
- **Details:** Minimal prod-style deployment (Compose or k3d). Includes FE, BE, DB, observability stubs.
- **Acceptance:** App boots in a minimal production environment.

### 🅅 Race-safe `ensure_user` hardening *(M)*
- **Details:** Handle unique-violation on insert by re-fetching; avoid duplicate users under concurrency.
- **Acceptance:** Concurrent first-login attempts never produce duplicate users or credentials.

### 🅆 Behavioral Improvements *(S → M)*
- **Email normalization** (trim, lowercase, Unicode NFKC).  
- **Email validation** (`422 INVALID_EMAIL`).  
- **Username hygiene** (min length/cleaning; store NULL if invalid).  
- **Last-login updates** (skip no-op writes).  
- **Error code catalog** (centralize codes).  
- **PII-safe logging** (mask/hash email and `google_sub`).  
- **Time provider abstraction** (injectable clock for deterministic tests).  
- **Rate limiting** (`429 RATE_LIMITED` on auth endpoint).

### 🅇 Frontend Import Hygiene & Lazy Loading *(S → M)*  
- **Consistent import ordering/grouping** (builtin, external, internal alias, parent/sibling, index).  
- **Type-only imports** enforced via ESLint.  
- **Dynamic `import()` / `next/dynamic`** for heavy libs or non-critical components.  
- **Example migration** of one component to `next/dynamic`.  
- **Docs** page explaining import policy and usage examples.  
