# 🃏 Nommie

Nommie is a **web-based, multiplayer version of Nomination Whist** (with our house rules).  
It’s a **full-stack, Docker-first app** with a clean split between frontend, backend, and database.

---

## Quick Start

1. Prereqs: Node 18+, pnpm 8+, Rust stable, Docker.
2. Copy env file and source it **once per shell**:
   - `cp .env.example .env`
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

**Important:** Environment variables must be sourced in each new shell session:
```bash
set -a; . ./.env; set +a
```
The project does not use `dotenvx` or `dotenvy` - all environment loading is done via shell sourcing.

### Key Environment Variables

**Database Configuration:**
- `POSTGRES_HOST`, `POSTGRES_PORT` - Database connection (defaults: localhost, 5432)
- `PROD_DB`, `TEST_DB` - Database names (test DB **must** end with `_test`)
- `APP_DB_USER`, `APP_DB_PASSWORD` - App role credentials
- `NOMMIE_OWNER_USER`, `NOMMIE_OWNER_PASSWORD` - Owner role credentials

**Backend Configuration:**
- `APP_JWT_SECRET` - JWT signing secret (required)
- `CORS_ALLOWED_ORIGINS` - Comma-separated allowed origins (defaults to localhost:3000, 127.0.0.1:3000)

**Frontend Configuration:**
- `BACKEND_BASE_URL` - Backend API URL (e.g., `http://localhost:3001`)
- `APP_JWT_SECRET` - NextAuth secret (shared with backend JWT secret from root .env)
- See Authentication Setup section for Google OAuth configuration

### Environment File Setup
1. **Root environment:** Copy `.env.example` to `.env` and update values
2. **Frontend environment:** See Authentication Setup section below for detailed frontend configuration
3. **Shared secrets:** The frontend automatically uses `APP_JWT_SECRET` from the root `.env` file

---

## Database & Migrations

**Auto-Migration**: Empty databases are automatically migrated on first connection via `build_state()`.

**Manual Migration Commands** (run with **Owner** role):
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

### Backend Testing (Nextest)

We use **cargo-nextest** as the primary test runner with sensible defaults.

**Running tests:**
- `pnpm be:test` - Run all tests (quiet by default)
- `pnpm be:test:v` - Verbose with success output at the end
- `pnpm be:test:q` - Quiet mode with final failure summary only
- `pnpm be:test:cargo` - Fallback to standard cargo test

**Targeted runs:**
- **Substring filter:** `pnpm be:test -- login` (runs tests with "login" in name)
- **File/module filter:** `pnpm be:test -- --test file_stem` (runs specific test file)
- **Expression filters:** `pnpm be:test -- -E 'status(fail)'` (runs only failing tests)
- **Preview what will run:**
  - `cargo nextest list --manifest-path apps/backend/Cargo.toml`
  - With filters: `cargo nextest list -E 'test(login)' --manifest-path apps/backend/Cargo.toml`

**Verbosity knobs:**
- `-q` / `-v` - Quiet/verbose output
- `--success-output=final` - Show success output at the end
- `--failure-output=final` - Show failure output at the end
- `--status-level` / `--final-status-level` - Control status display
- `--hide-progress-bar` - Disable progress bar
- `--no-capture` - Don't capture output (serializes execution)

**Opt-in logs:**
- Add `test_support::logging::init()` inside tests that need logs
- Enable levels with `TEST_LOG=info|debug|trace`
- Example: `TEST_LOG=info pnpm be:test:v -- some_filter`

Tests that hit the DB always use `TEST_DB` (guarded by `_test` suffix).

### Frontend Testing
- `fe:test` pending — will be added with Vitest + Testing Library.

---

## 🔐 Authentication Setup (NextAuth v5)

The frontend uses **NextAuth v5** with Google OAuth for user authentication.

### ⚙️ Environment Configuration
1. **Copy frontend env file:** `cp apps/frontend/.env.local.example apps/frontend/.env.local`
2. **Update required variables:**
   - `AUTH_GOOGLE_ID` & `AUTH_GOOGLE_SECRET`: Get from [Google Cloud Console](https://console.cloud.google.com/apis/credentials)
   - `BACKEND_BASE_URL`: Set to `http://localhost:3001` for local development
   - `APP_JWT_SECRET`: Already configured in root `.env` (shared with backend)

### 🔑 Google OAuth Setup
1. Go to [Google Cloud Console](https://console.cloud.google.com/apis/credentials)
2. Create OAuth 2.0 credentials for a web application
3. Add authorized redirect URI: `http://localhost:3000/api/auth/callback/google`
4. Copy Client ID and Client Secret to your `apps/frontend/.env.local` as `AUTH_GOOGLE_ID` and `AUTH_GOOGLE_SECRET`

### 🚀 Running with Authentication
- **Start the app:** `pnpm dev` (from root) or `pnpm dev:fe` (from `apps/frontend`)
- **Sign in:** Click "Sign in with Google" in the header
- **Protected routes:** `/dashboard` requires authentication
- **Sign out:** Click "Sign out" in the header when signed in

### 🛡️ Protected Routes
- `/dashboard/:path*` - User dashboard (requires auth)
- `/api/private/:path*` - Private API endpoints (requires auth)

---

## Auth Policy (Google OAuth)

- Each `user_credentials.email` is unique and links to at most one Google account.
- On login:
  - If `google_sub` is NULL, we set it to the incoming Google sub.
  - If `google_sub` is already set and equals the incoming sub, login succeeds and updates `last_login`.
  - If `google_sub` is already set and **differs** from the incoming sub, the request fails with:
    - **HTTP 409 CONFLICT**
    - Problem Details `code=GOOGLE_SUB_MISMATCH`
- We never silently overwrite `google_sub`. This prevents unintended or malicious account re-linking.
- Logging:
  - INFO on first creation and when setting `google_sub` from NULL.
  - DEBUG on repeat logins.
  - WARN on mismatch.

---

## 🏗️ Architecture
- **Frontend:** Next.js (App Router) + Tailwind CSS, NextAuth v5 (Google login)  
- **Backend:** Rust (Actix Web) + SeaORM 1.1.x, JWT validation  
- **Database:** PostgreSQL 16 (Docker Compose, schema via SeaORM migrator)  
- **Workflow:** pnpm workspaces, Docker-first, structured logs with trace IDs  

👉 See [Architecture & Tech Stack](docs/architecture-overview.md) for details.

## 📚 Documentation Index
- Architecture & Context: `docs/architecture-overview.md` (stack baseline), `docs/architecture-game-context.md` (request-scoped context model)
- Backend Operations: `docs/backend-error-handling.md` (RFC 7807 layers), `docs/backend-testing-guide.md` (DB harness & safeguards), `docs/backend-in-memory-game-engine.md` (AI simulation loop)
- Gameplay & AI: `docs/game-rules.md` (canonical rules), `docs/ai-player-implementation-guide.md` (production AI contract), `docs/game-snapshot-contract.md` (client payload shape)
- Frontend Experience: `docs/frontend-ui-roadmap.md` (UI delivery stages), `docs/frontend-theme-system.md` (semantic theme tokens)
- Delivery & Planning: `docs/project-milestones.md` (milestone tracking & optional enhancements)

---

## 🔒 Backend: Optimistic Concurrency

The backend uses optimistic locking with HTTP-native ETag/If-Match headers for safe concurrent updates.

### How It Works

1. **Reading Resources**: GET endpoints return an `ETag` header containing the resource version:
   ```
   ETag: "game-123-v5"
   ```

2. **Updating Resources**: PATCH/DELETE endpoints require an `If-Match` header with the last known ETag:
   ```
   If-Match: "game-123-v5"
   ```
   
   If the `If-Match` header is missing, the server returns `428 Precondition Required`.

3. **Conflict Detection**: If the resource has been modified since the client last read it, the server returns `409 Conflict`:
   ```json
   {
     "type": "https://nommie.app/errors/OPTIMISTIC_LOCK",
     "title": "Optimistic Lock",
     "status": 409,
     "code": "OPTIMISTIC_LOCK",
     "detail": "Resource was modified concurrently (expected version 5, actual version 6). Please refresh and retry.",
     "trace_id": "abc123",
     "extensions": {
       "expected": 5,
       "actual": 6
     }
   }
   ```

4. **Caching**: GET endpoints support `If-None-Match` for HTTP caching:
   - Single ETag: `If-None-Match: "game-123-v5"`
   - Multiple ETags: `If-None-Match: "game-123-v4", "game-123-v5"`
   - Wildcard: `If-None-Match: *` (matches any representation)
   
   If the client's ETag matches the current version, the server returns `304 Not Modified` with no body.

### Implementation Details

**For Developers Adding Mutation Endpoints:**

Use the `ExpectedVersion` extractor to automatically parse the `If-Match` header:

```rust
use crate::http::etag::{ExpectedVersion, game_etag};

async fn update_game(
    game_id: GameId,
    expected_version: ExpectedVersion, // Automatically parses If-Match
    payload: Json<UpdatePayload>,
) -> Result<HttpResponse, AppError> {
    // Use expected_version.0 when calling the repository
    game_repo::update(conn, game_id.0, expected_version.0, payload).await?;
    
    // Return new ETag in response
    Ok(HttpResponse::Ok()
        .insert_header((ETAG, game_etag(game_id.0, new_version)))
        .json(result))
}
```

**Observability:**

Optimistic lock conflicts are logged with structured fields for monitoring:

```rust
warn!(
    trace_id = %trace_id,
    expected = 5,
    actual = 6,
    "Optimistic lock conflict detected"
);
```

Use these logs to:
- Monitor conflict frequency (high rates may indicate UX issues)
- Correlate with specific operations or game states
- Debug race conditions in concurrent scenarios

**Architecture Notes:**

- Repositories and services choose connection types based on operation semantics:
  - Single reads and unrelated multi-reads: `ConnectionTrait` (accepts pool or transaction)
  - Related multi-reads (consistent snapshot): `DatabaseTransaction`
  - Any mutation: `DatabaseTransaction`
- All database operations go through `with_txn` or `require_db` for automatic transaction management
- Error handling follows [RFC 7807 Problem Details](https://www.rfc-editor.org/rfc/rfc7807) format
- The schema lives in a single init migration under `apps/backend/migration/`

---

## 🗺️ Roadmap
Milestone-driven: setup → core game loop → AI → polish.  
👉 See [Milestones](docs/project-milestones.md).

---

## 🎲 Game Rules
Gameplay house rules.  
👉 See [Game Rules](docs/game-rules.md).

---

## 📜 License
MIT
