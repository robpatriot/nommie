# Architecture Overview

## Purpose

Orients contributors to the high-level system shape: frontend, backend, data, and cross-cutting principles.

Subsystem deep-dives live in separate documents.

## System

Nommie is a web-based multiplayer implementation of Nomination Whist.

The system is Docker-first.

## Frontend

- Next.js (App Router)
- Tailwind CSS
- Authentication: NextAuth (Google login) issuing tokens consumed by the backend
- Package manager: pnpm

## Backend

- Rust
- Actix Web
- SeaORM (data access adapters only)

Backend layering:

- Domain: pure game logic
- Orchestration: DB + domain wiring
- Routes: thin HTTP adapters calling orchestration

## Data and Infrastructure

- PostgreSQL for production and normal development
- SQLite for targeted testing modes

Schema management:

- migrations are applied at startup as required by the runtime environment
- production schema readiness failures are treated as fatal startup errors

## Development Workflow

- lint and formatting are enforced via pnpm scripts
- tests include unit and integration coverage across frontend and backend
- logs are structured and include per-request trace identifiers

## Principles

- domain modules are infrastructure-free (no Actix or SeaORM)
- handlers return structured errors mapped to Problem Details responses
- extractors perform auth and request shaping, not business logic
- no panics in request handlers
