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
  *(Actual entities are in F.)*

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
  - Problem Details error shape:
    `{ type, title, status, detail, code, trace_id }`
  - `code` in SCREAMING_SNAKE.
  - Middleware injects per-request `trace_id`.
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

## 🟨 H — Extractors *(M → L, partially done)*
- **Dependencies:** E, F, G.
- **Completed:**
  - `AuthToken`
  - `JwtClaims`
  - `CurrentUser`
- **Remaining:**
  - `GameId` — validates bigint game ID and existence.
  - `GameMembership` — verifies membership/role in one hit where possible.
  - `ValidatedJson<T>` — shape validation with Problem Details errors.
- **Acceptance:** Handlers are thin; extractor tests pass; single DB hit for user+membership when possible.

---

## 🅘 I — Backend Domain Modules *(L)*
- **Dependencies:** G.
- **Details:**
  - Pure logic modules: `rules`, `bidding`, `tricks`, `scoring`, `state`.
  - No DB access in domain modules; orchestration above them.
- **Acceptance:** `grep` shows no SeaORM in domain modules.

---

## 🟨 J — Frontend App Router Seed *(M, partially done)*
- **Dependencies:** E, G.
- **Details:**
  - Next.js App Router + Turbopack.
  - Pages:
    - ✅ **Login** (NextAuth v5 wrapper for Google) — working.
    - ❌ **Lobby skeleton** — not yet implemented.
    - ❌ **Game skeleton** — not yet implemented.
- **Acceptance:** Can sign in, see lobby, and a placeholder game screen.

---

## 🅚 K — Game Lifecycle (Happy Path) *(L → XL)*
- **Dependencies:** H, G, I, J.
- **Details:**
  - End-to-end: `create → join → ready → deal → bid → trump → tricks → scoring → round advance`.
  - Integration test covers the minimal loop.
- **Acceptance:** Happy-path game completes.

---

## 🅛 L — AI Orchestration *(M → L)*
- **Dependencies:** J.
- **Details:**
  - Basic AI bidding + valid trick play.
  - Runs per poll cycle; AI auto-advances until human’s turn.
- **Acceptance:** Full game completes with AIs filling seats.

---

## 🅜 M — Validation, Edge Cases & Property Tests *(M)*
- **Dependencies:** J.
- **Details:**
  - Invalid bids/plays return proper Problem Details errors.
  - Property tests for trick/scoring invariants.
- **Acceptance:** Error paths validated; properties hold.

---

## 🅝 N — Frontend UX Pass (Round 1) *(M → L)*
- **Dependencies:** J, L.
- **Details:**
  - Hand display, trick area, bidding UI, trump selector.
  - FE surfaces Problem Details errors nicely.
- **Acceptance:** Gameplay is clear; errors understandable.

---

## 🟨 O — CI Pipeline *(S, partially done)*
- **Dependencies:** D, E, F, G, H, M, N.
- **Completed (local):**
  - Robust **pre-commit** hook: FE ESLint/Prettier (staged-aware), BE clippy + rustfmt (staged write).
- **Remaining (to complete CI gate):**
  - GitHub Actions workflow that gates PRs/`main` with:
    - FE: **ESLint**, **Prettier check**, **TypeScript typecheck**.
    - BE: **clippy**, **rustfmt --check**.
    - **Tests** with Postgres service; apply `init.sql` **twice**.
    - **Caching** (pnpm + Cargo).
- **Acceptance:** CI green gate (lint + typecheck + tests) required for merges; `init.sql` re-applies cleanly.

---

## 🅟 P — Documentation & Decision Log *(S)*
- **Dependencies:** J.
- **Details:**
  - README: setup + reset flow.
  - CONTRIBUTING: module layout, extractor policy, `_test` guard.
  - DECISIONS.md: locked decisions recorded.
- **Acceptance:** New devs onboard smoothly.

---

## 🅠 Q — Observability & Stability *(S → M)*
- **Dependencies:** E, J.
- **Details:**
  - Logs include `user_id` + `game_id` where relevant.
  - FE shows `trace_id` on errors.
  - Health endpoint reports DB status.
- **Acceptance:** Logs actionable; trace_id visible end-to-end.

---

## 🅡 R — Open Source Observability Stack *(M → L)*
- **Dependencies:** O, I.
- **Details:**
  - Grafana + Tempo + Loki + Prometheus in Docker.
- **Acceptance:** Infra captures app metrics, logs, and traces.

---

# 🔄 Optional Track (anytime)

### 🅂 WebSockets *(M)*
- **Dependencies:** J.
- **Details:** Replace polling with push (Actix WS or SSE). Ensure AI orchestration fits push model.
- **Acceptance:** FE receives live state; polling removed.

### 🅃 Transactional Tests *(S → M)*
- **Dependencies:** D.
- **Details:** Wrap integration tests in DB transactions; rollback for isolation; `_test` guard intact.
- **Acceptance:** Tests faster; DB clean post-run.

### 🅄 Deployment Stub *(S → M)*
- **Dependencies:** B, P, Q.
- **Details:** Minimal prod-like deployment (Compose or k3d). Includes FE, BE, DB, observability stubs.
- **Acceptance:** App boots in a minimal production-style environment.

### 🅅 Race-safe `ensure_user` hardening *(M)*
- **Details:** Handle unique-violation on insert by re-fetching; avoid duplicate users under concurrency.
- **Acceptance:** Concurrent first-login attempts never produce duplicate users or credentials.

### 🅆 Behavioral Improvements *(S → M)*
- **Email normalization**: normalize/truncate emails (trim + lowercase + Unicode NFKC).  
- **Email validation**: reject invalid addresses with `422 INVALID_EMAIL`.  
- **Username hygiene**: enforce min length/cleaning; store NULL if no valid username.  
- **Last-login updates**: avoid unnecessary writes if nothing changes.  
- **Error code catalog**: centralize SCREAMING_SNAKE codes in one module.  
- **PII-safe logging**: mask/hash email and google_sub in logs.  
- **Time provider abstraction**: trait-based clock injection for deterministic tests.  
- **Rate limiting**: middleware/gateway rate-limits on auth endpoint, with `429 RATE_LIMITED`.  
