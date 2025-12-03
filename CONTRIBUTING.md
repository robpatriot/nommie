# Contributing to Nommie

This document outlines the development workflow and conventions for contributing to Nommie.

## Development Setup

### Prerequisites
- Node.js 18+
- pnpm 8+
- Rust stable
- Docker

### Initial Setup
1. Clone the repository
2. Install dependencies: `pnpm i`
3. Copy and source environment:
   ```bash
   cp docs/env.example.txt .env
   set -a; . ./.env; set +a
   ```
4. Start PostgreSQL (run manually):
   ```bash
   docker compose -f docker/dev-db/docker-compose.yml up -d postgres
   ```
5. Create/refresh databases (run manually using migration-cli):
   ```bash
   # Dev database
   cargo run --bin migration-cli -- --env prod --db postgres fresh
   # Test database
   cargo run --bin migration-cli -- --env test --db postgres fresh
   ```

## Development Commands

### Backend
- **Start:** `pnpm be:up` (logs to `.dev/dev.log`)
- **Stop:** `pnpm be:down`
- **Build:** `pnpm be:build`
- **Test:** `pnpm be:test`
- **Lint:** `pnpm be:lint`
- **Format:** `pnpm be:format`

### Frontend
- **Start:** `pnpm fe:up`
- **Stop:** `pnpm fe:down`
- **Build:** `pnpm fe:build`
- **Lint:** `pnpm fe:lint`
- **Format:** `pnpm fe:format`

### Database
Run docker-compose commands manually:
- **Start:** `docker compose -f docker/dev-db/docker-compose.yml up -d postgres`
- **Stop:** `docker compose -f docker/dev-db/docker-compose.yml stop postgres`
- **Destroy:** `docker compose -f docker/dev-db/docker-compose.yml down -v`
- **Check readiness:** `pnpm db:svc:ready`
- **View logs:** `docker compose -f docker/dev-db/docker-compose.yml logs -f postgres`
- **Connect via psql:** `docker compose -f docker/dev-db/docker-compose.yml exec postgres psql -U "${POSTGRES_USER}" -d "${POSTGRES_DB}"`

For migrations, use the migration-cli binary:
- **Migrate prod DB:** `cargo run --bin migration-cli -- --env prod --db postgres up`
- **Fresh prod DB:** `cargo run --bin migration-cli -- --env prod --db postgres fresh`
- **Fresh test DB:** `cargo run --bin migration-cli -- --env test --db postgres fresh`

### Combined
- **Start all:** `pnpm start` (starts backend and frontend)
- **Stop all:** `pnpm stop` (stops backend and frontend)
- **Status:** `pnpm status` (shows backend and frontend status)
- **Lint all:** `pnpm lint`
- **Format all:** `pnpm format`

---

## Cursor Rules

This repo uses [Cursor](https://cursor.sh) for AI-assisted development.  
Project-specific conventions are locked in **`.cursor/rules.md`** — covering schema design, error handling, extractors, testing, and more.  

➡️ Always check that file before making changes; update it when project policies evolve.

---

## Environment Management

### Shell Sourcing
Environment variables must be sourced in your shell before running any commands:

```bash
set -a; . ./.env; set +a
```

**Important:** This must be done in each new shell session. The project does not use `dotenvx` or `dotenvy` - all environment loading is done via shell sourcing.

### Key Environment Variables
- `POSTGRES_HOST`, `POSTGRES_PORT` - Database connection
- `PROD_DB`, `TEST_DB` - Database names (test DB must end with `_test`)
- `APP_DB_USER`, `APP_DB_PASSWORD` - App role credentials
- `NOMMIE_OWNER_USER`, `NOMMIE_OWNER_PASSWORD` - Owner role credentials
- `APP_JWT_SECRET` - JWT signing secret
- `CORS_ALLOWED_ORIGINS` - Allowed CORS origins
- `ALLOWED_EMAILS` - (Optional) Comma-separated email allowlist for restricting signup/login. Supports glob patterns (e.g., `*@example.com`). If not set, all emails are allowed. Store in `.env.local` or deployment secrets (not committed).

## Code Conventions

### Rust (Backend)
- Follow `cargo fmt` and `cargo clippy` guidelines
- Use explicit error handling with `Result<T, AppError>`
- Domain logic stays pure (no DB/framework imports)
- Use enums over strings for states/roles/phases
- Prefer small, focused functions over large ones

### TypeScript/JavaScript (Frontend)
- Follow ESLint and Prettier configurations
- Prefer single formatted strings over concatenation

### Module Organization
- Module declarations at top of parent files (`mod`, `pub mod`)
- Grouped `use` statements at top of each file (std, extern, crate)
- `pub use` only in `lib.rs` or `prelude` modules

## Testing

### Backend Tests
- Run with: `pnpm be:test`
- Tests use `TEST_DB` (enforced `_test` suffix)
- All tests must be deterministic
- Use `StateBuilder` for test state creation

### Test Database Safety
- All destructive operations require `_test` suffix
- Never run destructive ops against production databases
- Use `MIGRATION_TARGET=test` for test database operations

## Database Migrations

Migrations run with the **Owner** role using the migration-cli binary:

- **Production:** `cargo run --bin migration-cli -- --env prod --db postgres up`
- **Test:** `cargo run --bin migration-cli -- --env test --db postgres up`
- **Fresh (prod):** `cargo run --bin migration-cli -- --env prod --db postgres fresh`
- **Fresh (test):** `cargo run --bin migration-cli -- --env test --db postgres fresh`

## Architecture Guidelines

### Backend Layering

The backend uses a **three-layer architecture** to separate concerns:

#### 1. Domain Layer (`apps/backend/src/domain/`)
**Purpose:** Pure game logic with no external dependencies.

**Rules:**
- ✅ Use standard library types (`Vec`, `Option`, `Result`)
- ✅ Define domain types (`GameState`, `Card`, `Bid`, `Phase`)
- ✅ Implement pure functions (`place_bid()`, `play_card()`, `apply_round_scoring()`)
- ❌ **No SeaORM imports** (no `Entity`, `Model`, `ActiveModel`)
- ❌ **No Actix Web imports** (no `HttpRequest`, `HttpResponse`)
- ❌ **No database imports** (no `ConnectionTrait`, `DatabaseTransaction`)

**Example:**
```rust
// ✅ Correct: Pure domain logic
pub fn place_bid(state: &mut GameState, who: PlayerId, bid: Bid) -> Result<(), DomainError> {
    // Game rules validation and state updates
}

// ❌ Incorrect: Domain module importing SeaORM
use sea_orm::Entity;  // Don't do this in domain/
```

#### 2. Orchestration Layer (`apps/backend/src/services/`, `apps/backend/src/repos/`)
**Purpose:** Coordinate domain logic with database operations.

**Rules:**
- ✅ Use SeaORM for database access (`repos/` modules)
- ✅ Call domain functions with domain types
- ✅ Orchestrate multi-step workflows (`services/` modules)
- ✅ Convert between database entities and domain types
- ❌ **No Actix Web HTTP types** (no `HttpRequest`, `HttpResponse` in services)
- ❌ **No direct database access in routes** (use repositories)

**Example:**
```rust
// ✅ Correct: Service orchestrates domain + DB
pub async fn place_bid(
    txn: &DatabaseTransaction,
    game_id: i32,
    seat: Seat,
    bid: Bid,  // Domain type
    lock_version: i32,
) -> Result<(), AppError> {
    let mut state = load_game_state(txn, game_id).await?;
    domain::bidding::place_bid(&mut state, seat, bid)?;  // Domain logic
    save_game_state(txn, game_id, &state, lock_version).await?;  // Persist
    Ok(())
}
```

#### 3. Routes Layer (`apps/backend/src/routes/`)
**Purpose:** Thin HTTP adapters that extract request data and call orchestration.

**Rules:**
- ✅ Use extractors for validation (`CurrentUser`, `GameId`, `ValidatedJson<T>`)
- ✅ Convert DTOs to domain types
- ✅ Call services/repositories (not domain directly from routes)
- ✅ Minimal logic; delegate to orchestration layer
- ❌ **No business logic in routes** (delegate to services)
- ❌ **No direct database access** (use repositories via services)

**Example:**
```rust
// ✅ Correct: Route extracts, validates, delegates
async fn submit_bid(
    body: ValidatedJson<BidRequest>,  // DTO with validation
    game_id: GameId,
    current_user: CurrentUser,
    // ...
) -> Result<HttpResponse, AppError> {
    let domain_bid = Bid(body.bid);  // Convert DTO → domain
    service.place_bid(txn, game_id.0, seat, domain_bid, body.lock_version).await?;
    Ok(HttpResponse::Ok().json(response_dto))
}
```

### Data Transfer Objects (DTOs)

DTOs are request/response types used at the HTTP boundary. They provide a stable contract between frontend and backend.

#### When to Create DTOs

**Request DTOs:**
- All POST/PATCH/PUT request bodies
- Must include `lock_version: i32` for mutation endpoints (optimistic locking)
- Use `ValidatedJson<T>` extractor for automatic JSON validation

**Response DTOs:**
- All JSON responses from API endpoints
- Separate from domain types (e.g., `GameSnapshotResponse` vs domain `GameSnapshot`)
- Use `snake_case` for JSON field names (Rust `snake_case` by default)

#### DTO Policies

1. **Separation from Domain:** DTOs are separate types, not aliases of domain types
   ```rust
   // ✅ Correct: Separate DTO
   #[derive(Deserialize)]
   struct BidRequest {
       bid: u8,
       lock_version: i32,
   }
   
   // Domain type (different purpose)
   pub struct Bid(pub u8);
   ```

2. **Validation:** Use extractors for HTTP-level validation; domain validates business rules
   ```rust
   // ✅ Correct: Extractors handle HTTP validation
   async fn submit_bid(
       body: ValidatedJson<BidRequest>,  // JSON parse errors → Problem Details
       // ...
   )
   ```

3. **Transformation:** Convert DTOs ↔ domain types in orchestration layer, not routes
   ```rust
   // ✅ Correct: Route converts DTO → domain, service uses domain
   let domain_bid = Bid(body.bid);
   service.place_bid(txn, game_id, seat, domain_bid, body.lock_version).await?;
   ```

4. **Optimistic Locking:** All mutation request DTOs must include `lock_version`
   ```rust
   #[derive(Deserialize)]
   struct UpdateGameRequest {
       // ... other fields ...
       lock_version: i32,  // Required for concurrent update safety
   }
   ```

#### Examples: Correct vs Incorrect

**✅ Correct Pattern:**
```rust
// Route: Extract and validate
async fn submit_bid(
    body: ValidatedJson<BidRequest>,  // DTO
    // ...
) -> Result<HttpResponse, AppError> {
    let domain_bid = Bid(body.bid);  // Convert to domain type
    service.place_bid(txn, game_id, seat, domain_bid, body.lock_version).await?;
    // ...
}

// Service: Orchestrate domain + DB
pub async fn place_bid(
    txn: &DatabaseTransaction,
    game_id: i32,
    seat: Seat,
    bid: Bid,  // Domain type
    lock_version: i32,
) -> Result<(), AppError> {
    let mut state = load_game_state(txn, game_id).await?;
    domain::bidding::place_bid(&mut state, seat, bid)?;  // Domain logic
    save_game_state(txn, game_id, &state, lock_version).await?;
    Ok(())
}

// Domain: Pure logic
pub fn place_bid(state: &mut GameState, who: PlayerId, bid: Bid) -> Result<(), DomainError> {
    // Pure game rules, no DB, no HTTP
}
```

**❌ Incorrect Pattern:**
```rust
// ❌ Route doing business logic
async fn submit_bid(body: ValidatedJson<BidRequest>, ...) -> ... {
    // Don't validate game rules here
    // Don't access database directly
    // Don't call domain functions directly
}

// ❌ Domain importing SeaORM
use sea_orm::Entity;  // Domain should be pure!

// ❌ Service using HTTP types
pub async fn place_bid(req: HttpRequest, ...) -> ... {  // Wrong layer!
}
```

### Config vs Infra Separation
- **Config** (`apps/backend/src/config/`): Pure environment parsing and URL construction
- **Infra** (`apps/backend/src/infra/`): Database connections and state building
- **Test Support** (`apps/backend/src/test_support/`): Test-only helpers

### Error Handling
- Use `AppError` with Problem Details format
- Include `trace_id` in all error responses
- Use `SCREAMING_SNAKE_CASE` error codes from central registry

### State Management
- `AppState` holds DB pool and `SecurityConfig`
- Inject via `web::Data<AppState>`
- Don't clone/rebuild security config ad-hoc

## Pull Request Process

1. Create a feature branch from `main`
2. Make your changes following the conventions above
3. Run tests: `pnpm be:test`
4. Run linters: `pnpm lint`
5. Run formatters: `pnpm format`
6. Ensure all tests pass
7. Submit a pull request with a clear description

## Questions?

Check the main [README.md](README.md) for more details about the project architecture and setup.
