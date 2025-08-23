# 🃏 Nommie

Nommie is a **web-based, multiplayer version of Nomination Whist** (with our house rules).  
It is built as a **full-stack, Docker-first application** with a clean split between frontend, backend, and database.

---

## �� Quick Start

1. **Install dependencies**:
   
       pnpm i

2. **Start frontend development server**:
   
       pnpm dev:fe
   
   Frontend will be available at [http://localhost:3000](http://localhost:3000)

3. **Start backend development server**:
   
       pnpm dev:be
   
   Backend will be available at [http://127.0.0.1:3001](http://127.0.0.1:3001)

4. **Run linting** (frontend ESLint + backend clippy):
   
       pnpm lint

5. **Run formatting** (frontend Prettier + backend fmt):
   
       pnpm format

---

## 🏗️ Architecture

- **Frontend:** Next.js (App Router) + Tailwind CSS, NextAuth v5 for Google login  
- **Backend:** Rust (Actix Web) + SeaORM, JWT validation  
- **Database:** PostgreSQL (managed by Docker Compose, schema via single init SQL)  
- **Workflow:** pnpm workspaces, Docker-first, structured logs with trace IDs  

👉 See [Architecture & Tech Stack](docs/architecture.md) for full details.

---

We run Postgres locally via Docker Compose.

### 1) Setup
cp .env.example .env
# Edit POSTGRES_PASSWORD in .env to a secure value

### 2) Start the database
pnpm run db:up
# Postgres runs in a container named nommie-postgres
# Bound to 127.0.0.1:5432
# Data is stored in the postgres_data volume

### 3) Stop the database (keep data)
pnpm run db:stop

### 4) Remove database + volume (wipe data)
pnpm run db:down

### 5) Logs & connectivity
pnpm run db:logs       # follow Postgres logs
pnpm run db:pg_isready # health check
pnpm run db:psql       # open psql shell inside container

---

## 🗺️ Roadmap

Development is milestone-driven, from setup → core game loop → AI → polish.  
👉 See [Milestones](docs/milestones.md) for the canonical TODO list.

---

## 🎲 Game Rules

Gameplay hosue rules.  
👉 See [Game Rules](docs/game-rules.md) for the authoritative ruleset.

---

## 📜 License

MIT
