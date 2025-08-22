# 🃏 Nommie

Nommie is a **web-based, multiplayer version of Nomination Whist** (with our house rules).  
It is built as a **full-stack, Docker-first application** with a clean split between frontend, backend, and database.

---

## 🚀 Quick Start

1. **Start the database** (Docker Compose):
   
       docker compose up -d postgres

2. **Install dependencies**:
   
       pnpm install

3. **Run frontend & backend together**:
   
       pnpm dev

---

## 🏗️ Architecture

- **Frontend:** Next.js (App Router) + Tailwind CSS, NextAuth v5 for Google login  
- **Backend:** Rust (Actix Web) + SeaORM, JWT validation  
- **Database:** PostgreSQL (managed by Docker Compose, schema via single init SQL)  
- **Workflow:** pnpm workspaces, Docker-first, structured logs with trace IDs  

👉 See [Architecture & Tech Stack](docs/architecture.md) for full details.

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
