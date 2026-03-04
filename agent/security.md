# Security Rules

- Never commit secrets or credentials.
- Use environment variables for sensitive configuration.
- Validate and sanitize all user input on the server.
- Do not rely on client-side validation.
- Use parameterized queries for database operations.
- Avoid leaking internal errors; return sanitized AppError Problem Details.
