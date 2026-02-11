# 🏗️ Nommie — Architecture & Tech Stack

## Document Scope

This overview orients new contributors to the high-level shape of the system:
frontend, backend, data, and the primary engineering principles that keep those
layers aligned. Deep-dives for specific subsystems live in separate documents
and are linked under **Related Documents**.

## 🌐 Overview

Nommie is a web-based, multiplayer version of **Nomination Whist**. The system 
is **full-stack** and **Docker-first**.

---

## 🎨 Frontend
- **Framework:** Next.js (App Router)
- **Styling:** Tailwind CSS
- **Auth:** NextAuth v5 beta (Google login + JWTs)
- **Build Tooling:** Turbopack (dev), standard Next.js build (prod)
- **Package Manager:** pnpm

---

## ⚙️ Backend
- **Language:** Rust
- **Framework:** Actix Web
- **ORM:** SeaORM (repositories in orchestration layer)
- **Auth:** JWT validation from NextAuth tokens
- **Architecture Layers:**
  - **Domain modules** → pure game logic (`rules`, `bidding`, `tricks`, `scoring`, `state`)
  - **Orchestration** → DB + domain wiring, per-feature modules (`orchestration::bidding`, etc.)
  - **Routes** → thin adapters that call orchestration

---

## 🗄️ Database & Infrastructure
- **Database:** PostgreSQL (production), SQLite (testing/local dev)
- **PostgreSQL:** Docker Compose manages Postgres (roles, DBs, grants, search_path)
- **SQLite:** In-memory for fast testing, file-based for local development
- **Schema Management:** SeaORM migrations with backend branching
- **Test DB:** programmatically recreated from init SQL at startup, `_test` guard enforced
- **Environment Variables:** - `SQLITE_DB_DIR`: Directory for SQLite file databases

---

## 🛠️ Dev Workflow
- **Testing:**
  - `pnpm test` runs all tests (unit + integration + smoke)
  - Property-based tests for tricky card logic (later milestones)

- **Lint & Format (pnpm scripts):**
  - `pnpm lint` → frontend lint + Prettier

- **Logging:**
  - Structured JSON logs
  - Per-request `trace_id`, surfaced in responses and logs

---

## 🧭 Principles
- **Docker-first** (host-pnpm for speed)
- **Init-only schema** — no runtime migrations
- **No panics in handlers** — all errors → Problem Details
  (`type`, `title`, `status`, `detail`, `code`, `trace_id`)
- **Extractors for authn/authz/shape** — not business rules
- **Domain-first design** — no SeaORM in domain modules

---

## Related Documents

- `architecture-game-context.md` — detailed design of the `GameContext`
  extractor and cache model.
- `backend-error-handling.md` — layered error strategy and RFC 7807 mapping.
- `backend-testing-guide.md` — database harness, safety rails, and test layout.
- `frontend-theme-system.md` — client experience
