# Dev Roadmap

This document tracks the current work plan for building the Nommie UI on web (Next.js) and, later, a mobile client. It captures stages and progress.

---

# Security enhancements

This section captures security-related issues and recommendations, grouped by priority.

## High Priority Issues

---

## Medium Priority Issues

### 1. Restricted Signup During Early Testing

**Issue:** During initial real-user testing, anyone who discovers the app could create an account, which is undesirable while the product and security posture are still stabilizing.

**Recommendation:**
- Gate signup to a controlled set of users (e.g., invite-only / email allowlist)
- Consider disabling open self-signup entirely and creating accounts via:
  - Admin-only endpoint or CLI
  - One-time invite tokens
- Add a clear toggle/flag in configuration to re-enable open signup later
- Ensure error messaging for blocked signup does not leak whether a given email is already registered

---

## Low Priority Issues / Best Practices

### 1. Logging and Monitoring

**Current State:**
- Backend logging:
  - PII redaction is centralized in `logging::pii` with `Redacted` wrappers for emails and tokens
  - `UserService` uses `Redacted(email)` and a helper to partially redact `google_sub` in auth-related logs
  - DB error adapters log errors with `raw_error = %Redacted(&error_msg)` and include `trace_id`
- Security events:
  - `logging::security::login_failed` emits structured events with `event="SECURITY_LOGIN_FAILED"`, `trace_id`, and redacted email
  - JWT verification logs `SECURITY_LOGIN_FAILED` for expired, invalid-signature, and invalid tokens

**Follow-ups (optional):**
- Extend `logging::security` with additional events (e.g., rate-limit hits) as new defenses are added
- Configure log aggregation/alerting to:
  - Monitor spikes in `SECURITY_LOGIN_FAILED`
  - Monitor DB availability errors and rate-limit events
  - Use `trace_id` consistently for correlation across services

### 2. Database Security

**Recommendations:**
- Ensure database connections use TLS in production
- Review database user permissions (least privilege principle)
- Enable database audit logging
- Regular database backups with encryption
- Consider using connection pooling limits (already implemented ✓)


