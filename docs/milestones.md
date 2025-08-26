# 🗺️ Nommie — Ordered Milestones Roadmap (Detailed)

## ✅ A — Repo & Project Bootstrap *(S, complete)*
- Monorepo created with `apps/frontend`, `apps/backend`, `packages/`.
- Root `.env` canonical; FE `.env.local` mirrors only `NEXT_PUBLIC_*`.
- Root `package.json` with scripts:
  - `backend:fmt` → `cargo fmt --manifest-path apps/backend/Cargo.toml --all`
  - `backend:clippy` → `cargo clippy --manifest-path apps/backend/Cargo.toml --all-targets --all-features -- -D warnings`
- ESLint/Prettier (FE) configured.
- Pre-commit hooks active.  
**Acceptance:** Hello-world FE/BE apps build locally; lint/format hooks pass.

---

## ✅ B — Docker-First Dev Environment *(M, complete)*
- Docker Compose with Postgres (roles, DBs, grants).
- Host-pnpm for speed; backend runs host or container.  
**Acceptance:** `pnpm dev` starts FE+BE; Postgres reachable; FE talks to BE.

---

## ✅ C — Database Schema via Init SQL *(M, complete)*
- Single SQL init file = source of truth.
- Test harness applies schema to `_test` DB at startup (guarded).  
**Acceptance:** Tests bootstrap schema cleanly; `_test` guard enforced.

---

## ✅ D — Testing Harness & Policies *(M, complete)*
- `pnpm test` runs all (unit + integration + smoke).
- Actix in-process integration test harness.
- First smoke test: create → add AI → snapshot.  
**Acceptance:** Tests green locally + CI.

---

## E — Error Shapes & Logging *(S → M)*
- Problem Details: `{ type, title, status, detail, code, trace_id }`.
- SCREAMING_SNAKE `code`s.
- Middleware adds per-request `trace_id`.
- Init tracing + structured JSON logs with `trace_id`.  
**Acceptance:** Consistent error responses; logs include trace_id.

---

## F — Extractors (Authn/Authz/Shape) *(M → L)*
- Extractors: `AuthToken`, `JwtClaims`, `CurrentUser`, `GameId`, `GameMembership`, `ValidatedJson<T>`.
- Ensure one DB hit across user + membership.
- Tests for each extractor.  
**Acceptance:** Handlers are thin; extractor tests pass.

---

## G — Backend Domain Modules *(L)*
- Pure logic modules under `src/game_management/`:
  - `rules` (round progression, schedule).
  - `bidding` (valid bids, dealer restriction, 3×0 rule).
  - `tricks` (follow suit, trick winner, turn order).
  - `scoring` (points + exact bid bonus).
  - `state` (phase transitions).
- No SeaORM references; functions accept/return plain Rust types.
- Unit tests for invariants.  
**Acceptance:** `grep` shows no SeaORM; domain modules tested independently.

---

## H — CI Pipeline *(S)*
- CI jobs for test + lint (required).
- Build optional until later.
- Cache config for Rust/Node modules.  
**Acceptance:** CI green gate required for merges.

---

## I — Frontend App Router Seed *(M)*
- Next.js App Router + Turbopack baseline.
- Lobby + Game pages skeleton.
- NextAuth v5 beta wrapper (Google login).
- Error handling wired to Problem Details.  
**Acceptance:** Can sign in, see lobby, placeholder game screen.

---

## J — Game Lifecycle (Happy Path) *(L → XL)*
- End-to-end flow:
  - Create → join → ready → deal → bid → trump → tricks → scoring → round advance.
- Orchestration layer ties domain (G) to persistence (SeaORM).
- Integration test covers the full cycle (one game).  
**Acceptance:** Happy-path game completes.

---

## K — AI Orchestration *(M → L)*
- Minimal AI bidding + trick play.
- Runs each poll cycle; respects rules.
- Fills empty seats if humans < 4.  
**Acceptance:** Full game completes with AIs filling seats.

---

## L — Validation, Edge Cases & Property Tests *(M)*
- Reject invalid bids/plays cleanly with Problem Details.
- Property tests for invariants:
  - One trick winner always.
  - Must follow suit if possible.
  - Round schedule matches 26-round arc.
- Negative path tests for extractors/handlers.  
**Acceptance:** Error paths + properties tested.

---

## M — Documentation & Decision Log *(S)*
- `README.md`: setup, reset flow.
- `CONTRIBUTING.md`: module layout, extractor policy, `_test` guard.
- `DECISIONS.md`: record locked choices (rules, schema, milestones).  
**Acceptance:** New devs onboard smoothly.

---

## N — Frontend UX Pass (Round 1) *(M → L)*
- Hand display, trick area, bidding UI, trump selector.
- Surface backend Problem Details to FE nicely.
- Show player turns, highlight illegal actions.  
**Acceptance:** Gameplay is clear; errors understandable.

---

## O — Observability & Stability *(S → M)*
- Logs include `user_id` + `game_id` when relevant.
- FE shows `trace_id` on errors.
- Health endpoint reports DB status.  
**Acceptance:** Logs actionable; trace_id visible end-to-end.

---

## P — Open Source Observability Stack *(M → L)*
- Self-host OSS stack: Grafana + Tempo + Loki + Prometheus + OTel Collector.
- App exports OTLP traces/metrics/logs to Collector.
- Grafana dashboards link metrics → traces → logs.
- RED metrics (Rate, Errors, Duration) charted per endpoint.  
**Acceptance:** Grafana shows traces (`service.name=nommie-backend`), logs link to traces, basic metrics visible.

---

# 🔄 Optional Track (anytime)

### 1. WebSockets *(M)*
- Server push for snapshots (replace polling).
- FE subscription model.  
**Acceptance:** Polling replaced with push.

### 2. Transactional Tests *(S → M)*
- Try per-test rollback isolation in harness.  
**Acceptance:** Faster tests, no flakiness.

### 3. Deployment Stub *(S → M)*
- Container images + minimal prod runtime config.
- Health endpoint available.  
**Acceptance:** App boots in prod mode with init-only schema.
