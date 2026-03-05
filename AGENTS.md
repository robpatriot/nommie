# AGENTS.md — Nommie Agent Contract (Canonical)

This is the primary instruction file for AI agents working in this repo.

If any instruction conflicts with this file: stop and ask.

## Prime directive
- Prefer correctness, clarity, and maintainability over speed.
- Follow existing patterns before introducing new abstractions.
- Keep changes goal-scoped: avoid unrelated refactors or cosmetic churn.

## Non-negotiables
- Never commit secrets or credentials.
- Don’t add “legacy/compat” shims: refactor callers instead.
- Don’t leave prompt artifacts or implementation plans in code comments.
- When a rule cannot be followed, explain the conflict and propose the compliant alternative.

## How to run
Use `package.json` scripts as canonical. Prefer inspecting scripts over guessing commands.

Common patterns include: install (`pnpm install`), dev (`pnpm dev`), lint (`pnpm lint`), test (`pnpm test`), build (`pnpm build`).

## Repo map
- Frontend: `apps/frontend` (Next.js App Router)
- Backend: `apps/backend` (Actix Web + SeaORM)
- Shared TS: `packages/`
- Human docs: `docs/`
- Agent guidance: `agent/`

## Scoped entrypoints
- Backend: read `agent/backend/index.md`
- Frontend: read `agent/frontend/index.md`

## Topic guides
- Workflow (moves, comments, commits): `agent/workflow.md`
- Security: `agent/security.md`
- Testing: `agent/testing.md`
- Backend deep rules: `agent/backend/*`
- Frontend deep rules: `agent/frontend/*`
- Documentation: `agents/docs.md`
