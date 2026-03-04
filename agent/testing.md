# Testing Rules

- Tests must be deterministic.
- Add tests for complex logic and edge/error paths.
- Prefer structured assertions over string matching.
- Use repository scripts in package.json as canonical for running tests, lint, and build.
- If behavior changes, ensure relevant test coverage exists or is updated.
