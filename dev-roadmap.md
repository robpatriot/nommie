# Dev Roadmap

This document tracks the current work plan for building the Nommie UI on web (Next.js) and, later, a mobile client. It captures stages and progress.

---

# Next Steps

- Implement process for restricted users - security enhancement 1 below
- Run lint and tests and commit
- Fix eslint config
- Fix initial load of lobby games list
- Complete mobile design - Milestone 17

## Security enhancements

This section captures security-related issues and recommendations.

### 1. Restricted Signup During Early Testing

**Issue:** During initial real-user testing, anyone who discovers the app could create an account, which is undesirable while the product and security posture are still stabilizing.

**Recommendation:**
- Gate signup AND login to a controlled set of users
- Use email allowlist env var - if exists then it applies
- env var should be comma separated and allow globs like *@blah.com
- If server starts and env var exists invalidate logins for all users not in the list 
- Ensure error messaging for blocked signup does not leak whether a given email is already registered
- Note: will add an admin only toggle/flag to re-enable open signup later
