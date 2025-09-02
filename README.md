# 🃏 Nommie

Nommie is a **web-based, multiplayer version of Nomination Whist** (with our house rules).  
It’s a **full-stack, Docker-first app** with a clean split between frontend, backend, and database.

---

## 🚀 Quick Start
1. **Install deps:** `pnpm i`
2. **Frontend:** `pnpm dev:fe` → [http://localhost:3000](http://localhost:3000)
3. **Backend:** `pnpm dev:be` → [http://127.0.0.1:3001](http://127.0.0.1:3001)
4. **Lint:** `pnpm lint`
5. **Format:** `pnpm format`

---

## Cursor Rules

This repo uses [Cursor](https://cursor.sh) for AI-assisted development.  
Project-specific conventions are locked in **`.cursor/rules.md`** — covering schema design, error handling, extractors, testing, and more.  

➡️ Always check that file before making changes; update it when project policies evolve.

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

## 🗄️ Database

We run Postgres locally via Docker Compose.

### ⚙️ Setup
```bash
cp docs/env.example.txt .env
# Edit APP_DB_PASSWORD and POSTGRES_PASSWORD to secure values
```

### ▶️ Start / Stop / Destroy
```bash
pnpm db:up      # start Postgres (container: nommie-postgres)
pnpm db:stop    # stop container, keep data
pnpm db:down    # stop + remove container & volume (wipe data)
```
- Bound to `127.0.0.1:5432`  
- Data stored in `postgres_data` volume

### 🔍 Logs & Connectivity
```bash
pnpm db:logs       # follow logs
pnpm db:pg_isready # check health
pnpm db:psql       # open psql shell
```

### 🗂️ Schema (SeaORM migrator)
```bash
pnpm db:migrate    # apply new migrations (safe)
pnpm db:fresh      # drop + rebuild schema in dev DB (nommie)
pnpm db:fresh:test # drop + rebuild schema in test DB (nommie_test)
```

### 🧹 Clean-room Recipes
**Dev DB**
```bash
pnpm db:down && pnpm db:up
pnpm db:pg_isready
pnpm db:fresh
```
**Test DB**
```bash
pnpm db:fresh:test
pnpm test
```

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
