# 🃏 Nommie

Nommie is a **web-based, multiplayer version of Nomination Whist** (with our house rules).  
It’s a **full-stack, Docker-first app** with a clean split between frontend, backend, and database.

---

## Quick Start

1. Prereqs: Node 18+, pnpm 8+, Rust stable, Docker.
2. Copy env and source it **once per shell**:
   - `cp docs/env.example.txt .env`
   - `set -a; . ./.env; set +a`
3. Start Postgres:
   - `pnpm db:up`
4. Create/refresh databases:
   - Dev DB (owner role): `pnpm db:fresh`
   - Test DB (owner role): `pnpm db:fresh:test`
5. Run backend + frontend:
   - Backend: `pnpm be:up` (logs → `.dev/dev.log`, stop with `pnpm be:down`)
   - Frontend: `pnpm fe:up` (stop with `pnpm fe:down`)
6. Run backend tests:
   - `pnpm be:test` (plain `cargo test --nocapture` for now)

> Tip: If a shell is new, re-source env: `set -a; . ./.env; set +a`

## Environment

We don't store `DATABASE_URL`. We store **parts** in `.env` and construct URLs in code.

- Source env in your shell before running anything:
  - `set -a; . ./.env; set +a`
- Key vars (see `docs/env.example.txt`):
  - `POSTGRES_HOST`, `POSTGRES_PORT`
  - `PROD_DB`, `TEST_DB` (test DB **must** end with `_test`)
  - App role: `APP_DB_USER`, `APP_DB_PASSWORD`
  - Owner role: `NOMMIE_OWNER_USER`, `NOMMIE_OWNER_PASSWORD`
  - `APP_JWT_SECRET`, `CORS_ALLOWED_ORIGINS`

---

## Database & Migrations

Migrations run with the **Owner** role. Choose a target DB via `MIGRATION_TARGET`.

- Migrate prod DB:
  - `pnpm db:migrate`  (equivalent to `MIGRATION_TARGET=prod … -- up`)
- Fresh prod DB:
  - `pnpm db:fresh`
- Fresh test DB:
  - `pnpm db:fresh:test` (uses `MIGRATION_TARGET=test`)
- Readiness helpers:
  - `pnpm db:pg_isready`
  - `pnpm db:psql`

---

## Testing

Backend (current):
- `pnpm be:test` → runs `cargo test -- --nocapture`
- Tests that hit the DB always use `TEST_DB` (guarded by `_test` suffix).

Frontend:
- `fe:test` pending — will be added with Vitest + Testing Library.

---

## 🔐 Authentication Setup (NextAuth v5)

The frontend uses **NextAuth v5** with Google OAuth for user authentication.

### ⚙️ Environment Configuration
1. **Copy the example file:** `cp apps/frontend/.env.example apps/frontend/.env.local`
2. **Edit `.env.local`** with your actual values:
   - `GOOGLE_CLIENT_ID` & `GOOGLE_CLIENT_SECRET`: Get from [Google Cloud Console](https://console.cloud.google.com/apis/credentials)
   - `NEXTAUTH_SECRET`: Generate with `openssl rand -base64 32`
   - `NEXTAUTH_URL`: Set to `http://localhost:3000` for local development

### 🔑 Google OAuth Setup
1. Go to [Google Cloud Console](https://console.cloud.google.com/apis/credentials)
2. Create OAuth 2.0 credentials for a web application
3. Add authorized redirect URI: `http://localhost:3000/api/auth/callback/google`
4. Copy Client ID and Client Secret to your `.env.local`

### 🚀 Running with Authentication
- **Start the app:** `pnpm dev` (from root) or `pnpm dev:fe` (from `apps/frontend`)
- **Sign in:** Click "Sign in with Google" in the header
- **Protected routes:** `/dashboard` requires authentication
- **Sign out:** Click "Sign out" in the header when signed in

### 🛡️ Protected Routes
- `/dashboard/:path*` - User dashboard (requires auth)
- `/api/private/:path*` - Private API endpoints (requires auth)

---

## 🏗️ Architecture
- **Frontend:** Next.js (App Router) + Tailwind CSS, NextAuth v5 (Google login)  
- **Backend:** Rust (Actix Web) + SeaORM 1.1.x, JWT validation  
- **Database:** PostgreSQL 16 (Docker Compose, schema via SeaORM migrator)  
- **Workflow:** pnpm workspaces, Docker-first, structured logs with trace IDs  

👉 See [Architecture & Tech Stack](docs/architecture.md) for details.

---

## 🗺️ Roadmap
Milestone-driven: setup → core game loop → AI → polish.  
👉 See [Milestones](docs/milestones.md).

---

## 🎲 Game Rules
Gameplay house rules.  
👉 See [Game Rules](docs/game-rules.md).

---

## 📜 License
MIT
