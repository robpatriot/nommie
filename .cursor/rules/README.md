# Cursor Rules

This folder contains modular rules for Cursor.

- `global.mdc`: Workspace-wide rules (applies to all files via `globs: ["**/*"]`)
- `backend.mdc`: Rules scoped to `apps/backend/**`
- `general-best-practices.mdc`: General best practices for security, testing, error handling, and AI-assisted development

All files use MDC (Markdown with Context) format with:
- YAML frontmatter: `description`, `globs`, and `alwaysApply: true`
- Markdown content for clear, structured rules

Rules automatically load at session start via `alwaysApply: true`. The `globs` field determines which files trigger the rule.

Extend by adding new `*.mdc` files with `alwaysApply: true` and appropriate `globs` patterns.
