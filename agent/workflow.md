# Workflow Rules

## Change discipline
- Keep changes goal-scoped: avoid unrelated refactors or cosmetic churn.
- Follow existing code style and architecture.
- If behavior changes, update relevant human docs under docs/.

## File moves
- Use git mv for tracked files.
- Do not recreate files when renaming or moving.

## Code comments
Comments must:
- Explain rationale, invariants, and non-obvious behavior.

Comments must not:
- Include implementation plans or step lists from prompts.
- Include historical change logs.
- Include prompt artifacts.

## Commits (when asked)
If asked to propose commits:
- Provide a short list of proposed commits with file list, message, and rationale.
- Do not execute commits unless explicitly told to do so.
