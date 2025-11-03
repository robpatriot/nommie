# Cursor Rules

This folder contains modular rules for Cursor.

- `global.yaml`: Workspace-wide rules
- `backend.yaml`: Rules scoped to `apps/backend/**`

All files use YAML:
- `version: 1`
- Optional `match` for path scoping
- `rules`: list of `{ id, text, [scope] }`

Extend by adding new `*.yaml` files with `match` as needed.
