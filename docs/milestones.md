# 🗺️ Nommie — Ordered Milestones Roadmap (Updated, Detailed)

## ✅ A — Repo & Project Bootstrap  
- Monorepo created with `apps/frontend`, `apps/backend`, `packages/`.  
- Root `.env` canonical; FE `.env.local` mirrors only `NEXT_PUBLIC_*`.  
- **Root `package.json`** created with scripts:  
  - `backend:fmt` → `cargo fmt --manifest-path apps/backend/Cargo.toml --all`  
  - `backend:clippy` → `cargo clippy --manifest-path apps/backend/Cargo.toml --all-targets --all-features -- -D warnings`  
- ESLint/Prettier (FE) configured.  
- Pre-commit hooks active.  

**Acceptance:** Hello-world FE/BE apps build locally; lint/format hooks pass.

---

## ✅ B — Docker-First Dev Environment  
- Docker Compose with Postgres (roles, DBs, grants).  
- Host-pnpm for speed; backend runs host or container.  

**Acceptance:** `pnpm dev` starts FE+BE; Postgres reachable; FE talks to BE.

---

## ✅ C — Database Schema via Init SQL (Scaffolding Only)  
- Single SQL init file = source of truth.  
- Test harness applies schema to `_test` DB at startup (guarded).  

**Acceptance:** Tests bootstrap schema cleanly; `_test` guard enforced.  
*(Note: actual entities deferred to new Milestone F.)*

---

## ✅ D — Testing Harness & Policies  
- `pnpm test` runs all (unit + integration + smoke).  
- Actix in-process integration test harness.  
- First smoke test: create → add AI → snapshot.  

**Acceptance:** Tests green locally + CI.

---

## ✅ E — Error Shapes & Logging *(S → M)*  
- **Dependencies:** D  
- **Details:**  
  - Problem Details shape: `{ type, title, status, detail, code, trace_id }`.  
  - `code` = SCREAMING_SNAKE.  
  - Middleware adds per-request `trace_id`.  
- **Acceptance:** Consistent error responses; logs include `trace_id`.

---

## ✅ F — Database Schema (Actual Entities) *(M → L)*  
- **Dependencies:** C (plumbing done), D (tests in place).  
- **Details:**  
  - Define real schema in `init.sql`.  
  - Tables: `users`, `games`, `memberships`, `bids`, `plays`, `scores`.  
  - Add enums for state/roles where needed.  
  - Foreign keys + indexes for performance.  
- **Acceptance:**
  - `init.sql` defines canonical schema.  
  - Tests reset and apply schema cleanly.  
  - Entities present and coherent with game lifecycle needs.

---

## 🅖 G — User Authentication *(M → L)*  
- **Dependencies:** F  
- **Details:**  
  - Use **Google OAuth** for account creation and authentication.  
  - Use **JWTs** for frontend/backend authentication.  
  - Add an **auth extractor** to handle JWT authentication and user session.  
- **Acceptance:**  
  - Users can authenticate using **Google** OAuth.  
  - JWTs are used for frontend/backend authentication.  
  - Extractor handles JWT validation and user session.

---

## 🅗 H — Extractors *(M → L)*  
- **Dependencies:** E, F  
- **Details:**  
  - Extractors: `AuthToken`, `JwtClaims`, `CurrentUser`, `GameId`, `GameMembership`, `ValidatedJson<T>`.  
  - Ensure single DB hit for user + membership.  
- **Acceptance:** Handlers are thin; extractor tests pass.

---

## 🅘 I — Backend Domain Modules *(L)*  
- **Dependencies:** G  
- **Details:**  
  - Pure logic in `rules`, `bidding`, `tricks`, `scoring`, `state`.  
  - No DB access in domain modules. Orchestration sits on top.  
- **Acceptance:** `grep` shows no SeaORM in domain modules.

---

## 🅙 J — CI Pipeline *(S)*  
- **Dependencies:** D, E, F, G, H  
- **Details:**  
  - CI jobs: **test** + **lint** required; build optional until later.  
- **Acceptance:** CI green gate required for merges.

---

## 🅚 K — Frontend App Router Seed *(M)*  
- **Dependencies:** E, G  
- **Details:**  
  - Next.js App Router + Turbopack.  
  - Lobby + Game pages skeleton.  
  - NextAuth v5 beta wrapper.  
- **Acceptance:** Can sign in, see lobby, placeholder game screen.

---

## 🅛 L — Game Lifecycle (Happy Path) *(L → XL)*  
- **Dependencies:** H, G, I, J  
- **Details:**  
  - End-to-end game: create → join → ready → deal → bid → trump → tricks → scoring → round advance.  
  - Integration test covers minimal loop.  
- **Acceptance:** Happy-path game completes.

---

## 🅜 M — AI Orchestration *(M → L)*  
- **Dependencies:** K  
- **Details:**  
  - Basic AI bidding + trick play.  
  - Runs per poll cycle.  
- **Acceptance:** Full game completes with AIs filling seats.

---

## 🅝 N — Validation, Edge Cases & Property Tests *(M)*  
- **Dependencies:** K  
- **Details:**  
  - Invalid bids/plays return proper errors.  
  - Property tests for trick/scoring invariants.  
- **Acceptance:** Error paths + properties tested.

---

## 🅞 O — Documentation & Decision Log *(S)*  
- **Dependencies:** K (so docs reflect reality).  
- **Details:**  
  - README: setup, reset flow.  
  - CONTRIBUTING: module layout, extractor policy, `_test` guard.  
  - DECISIONS.md: locked decisions recorded.  
- **Acceptance:** New devs onboard smoothly.

---

## 🅟 P — Frontend UX Pass (Round 1) *(M → L)*  
- **Dependencies:** K, M  
- **Details:**  
  - Hand display, trick area, bidding UI, trump selector.  
  - FE surfaces Problem Details errors nicely.  
- **Acceptance:** Gameplay clear; errors understandable.

---

## 🅠 Q — Observability & Stability *(S → M)*  
- **Dependencies:** E, K  
- **Details:**  
  - Logs include `user_id` + `game_id` when relevant.  
  - FE shows `trace_id` on errors.  
  - Health endpoint with DB status.  
- **Acceptance:** Logs actionable; trace_id visible end-to-end.

---

## 🅡 R — Open Source Observability Stack *(M → L)*  
- **Dependencies:** P, I  
- **Details:**  
  - Grafana + Tempo + Loki + Prometheus in Docker.  
- **Acceptance:** Infra captures app metrics/logs/traces.

---

### **Optional Track (can be done anytime)**:

### 1. **WebSockets** *(M)*  
- **Dependencies:** K  
- **Notes:** Easier once lifecycle exists, but orthogonal.

### 2. **Transactional Tests** *(S → M)*  
- **Dependencies:** D  
- **Notes:** Optimization, not essential.

### 3. **Deployment Stub** *(S → M)*  
- **Dependencies:** B, P  
- **Notes:** Minimal prod bootstrapping; flexible timing.
