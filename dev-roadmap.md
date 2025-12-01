# Dev Roadmap

This document tracks the current work plan for building the Nommie UI on web (Next.js) and, later, a mobile client. It captures stages and progress.

---

# Security enhancements

This section captures security-related issues and recommendations.

### 1. Restricted Signup During Early Testing

**Issue:** During initial real-user testing, anyone who discovers the app could create an account, which is undesirable while the product and security posture are still stabilizing.

**Recommendation:**
- Gate signup to a controlled set of users (e.g., invite-only / email allowlist)
- Consider disabling open self-signup entirely and creating accounts via:
  - Admin-only endpoint or CLI
  - One-time invite tokens
- Add a clear toggle/flag in configuration to re-enable open signup later
- Ensure error messaging for blocked signup does not leak whether a given email is already registered

### 2. Database Security

**Recommendations:**
- Ensure database connections use TLS in production
- Review database user permissions (least privilege principle)
- Enable database audit logging
- Regular database backups with encryption
- Consider using connection pooling limits (already implemented ✓)


