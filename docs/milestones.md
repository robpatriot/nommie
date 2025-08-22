# 🗺️ Nommie — Milestones Roadmap

This roadmap combines clean-slate planning with lessons learned from Nommie v1.  
Lettering is for navigation, not strict sequence — some may run in parallel.

---

## 🅰️ Milestone A — Repo & Project Bootstrap
- Monorepo created with `apps/frontend`, `apps/backend`, `packages/`  
- Root `.env` canonical; FE `.env.local` mirrors only `NEXT_PUBLIC_*`  
- Four ChatGPT prompts seeded (architecture, ways-of-working, milestones, game-rules)  
- ESLint/Prettier (FE), `pnpm backend:fmt` / `pnpm backend:clippy` (BE)  
- Pre-commit hooks active  

**Acceptance:** Hello-world FE/BE apps build locally; lint/format hooks pass.

---

## 🅱️ Milestone B — Docker-First Dev Environment
- Docker Compose with Postgres (roles, DBs, grants)  
- Host-pnpm for speed; backend runs host or container  

**Acceptance:** `pnpm dev` starts FE+BE; Postgres reachable; FE talks to BE.

---

## 🇨 Milestone C — Database Schema via Init SQL
- Single SQL init file = source of truth  
- Test harness applies schema to `_test` DB at startup (guarded)  

**Acceptance:** Tests bootstrap schema cleanly; `_test` guard enforced.

---

## 🇩 Milestone D — Testing Harness & Policies
- `pnpm test` runs all (unit + integration + smoke)  
- Actix in-process integration test harness  
- First smoke test: create → add AI → snapshot  

**Acceptance:** Tests green locally + CI.

---

## 🇪 Milestone E — Error Shapes & Logging
- Problem Details: `{ type, title, status, detail, code, trace_id }`  
- SCREAMING_SNAKE `code`s  
- Middleware adds per-request `trace_id`  

**Acceptance:** Consistent error responses; logs include trace_id.

---

## 🇫 Milestone F — Extractors (Authn/Authz/Shape)
- Extractors: `AuthToken`, `JwtClaims`, `CurrentUser`, `GameId`, `GameMembership`, `ValidatedJson<T>`  
- Single DB hit across user + membership  

**Acceptance:** Handlers are thin; extractor tests pass.

---

## 🇬 Milestone G — Backend Domain Modules
- Pure logic in `rules`, `bidding`, `tricks`, `scoring`, `state`  
- Orchestration per feature  

**Acceptance:** grep shows no SeaORM in domain modules.

---

## 🇭 Milestone H — CI Pipeline
- CI jobs: **test** + **lint** required; build optional until later  

**Acceptance:** CI green gate required for merges.

---

## 🇮 Milestone I — Frontend App Router Seed
- Next.js App Router + Turbopack  
- Lobby + Game pages skeleton  
- NextAuth v5 beta wrapper  

**Acceptance:** Can sign in, see lobby, placeholder game screen.

---

## 🇯 Milestone J — Game Lifecycle (Happy Path)
- End-to-end game: create → join → ready → deal → bid → trump → tricks → scoring → round advance  
- Integration test covers minimal loop  

**Acceptance:** Happy-path game completes.

---

## 🇰 Milestone K — AI Orchestration
- Basic AI bidding + trick play  
- Runs per poll cycle  

**Acceptance:** Full game completes with AIs filling seats.

---

## 🇱 Milestone L — Validation, Edge Cases & Property Tests
- Invalid bids/plays return proper errors  
- Property tests for trick/scoring invariants  

**Acceptance:** Error paths + properties tested.

---

## 🇲 Milestone M — Documentation & Decision Log
- README: setup, reset flow  
- CONTRIBUTING: module layout, extractor policy, `_test` guard  
- DECISIONS.md: locked decisions recorded  

**Acceptance:** New devs onboard smoothly.

---

## 🇳 Milestone N — Frontend UX Pass (Round 1)
- Hand display, trick area, bidding UI, trump selector  
- FE surfaces Problem Details errors nicely  

**Acceptance:** Gameplay clear; errors understandable.

---

## 🇴 Milestone O — Observability & Stability
- Logs include `user_id` + `game_id` when relevant  
- FE shows `trace_id` on errors  
- Health endpoint with DB status  

**Acceptance:** Logs actionable; trace_id visible end-to-end.

---

## 🇵 Milestone P — WebSockets (optional later)
- Server push for snapshots → lower latency  

**Acceptance:** Polling replaced with push updates.

---

## 🇶 Milestone Q — Transactional Tests (Evaluation)
- Try per-test rollback isolation  

**Acceptance:** Faster test runtime without flakiness.

---

## 🇷 Milestone R — Deployment Stub (optional later)
- Container images + prod runtime config  

**Acceptance:** App boots in prod mode with init-only schema.
