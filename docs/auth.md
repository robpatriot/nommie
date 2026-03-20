# Authentication

## Scope

This document specifies the authentication model for Nommie.

It defines:

- identity provider integration
- session token lifecycle
- cookie conventions
- backend session validation
- WebSocket authentication
- logout and revocation
- frontend API call authentication

## Identity Provider

Authentication is delegated to Google OAuth via NextAuth.js.

The NextAuth session (stored as a signed JWT cookie) is managed by Next.js and is not visible to the backend.

The backend has no knowledge of Google OAuth. It authenticates requests using opaque session tokens only.

## Backend Session Tokens

On successful Google sign-in, the frontend calls the backend login endpoint:

```
POST /api/auth/login
{ email, name, google_sub }
```

The backend:

1. Checks the email allowlist (if configured).
2. Creates or updates the user record.
3. Generates a cryptographically random opaque session token (UUID v4 simple format — 32 lowercase hex characters).
4. Stores `SessionData` in Redis under `session:<token>` with a 24-hour TTL.
5. Returns `{ token }` in the response body.

The frontend stores the returned token in the `backend_session` cookie.

### SessionData

```
user_id: i64   — database user ID
sub:     String — external identity (google_sub)
email:   String
```

### Token Properties

- 128 bits of entropy (UUID v4)
- opaque to the client
- no expiry information encoded in the token itself
- TTL is server-side only

## Cookie Convention

The session token is transmitted as an HttpOnly cookie.

```
Name:     backend_session
HttpOnly: true
SameSite: Lax
Secure:   true (production)
MaxAge:   86400 (24 hours)
Path:     /
```

The `backend_session` cookie is set by the frontend server after login and forwarded to the backend on every authenticated API call.

## Backend Session Validation

All protected backend routes use the `SessionExtract` middleware.

On each request, `SessionExtract`:

1. Reads the `backend_session` cookie value.
2. Looks up `session:<token>` in Redis.
3. Returns 401 if the token is missing or not found.
4. Returns 503 if Redis is unavailable.
5. Slides the session TTL by resetting it to 24 hours.
6. Inserts `SessionData` into request extensions for downstream extractors.

The `CurrentUser` extractor reads `SessionData` from request extensions, then performs a single database lookup by `user_id` to fetch the user's `username` and `role`. These fields are not stored in `SessionData` and must come from the database.

## Frontend API Authentication

Server-side API calls (server actions, route handlers, server components) read the `backend_session` cookie and include it as a `Cookie` header in requests to the backend:

```
Cookie: backend_session=<token>
```

No `Authorization` header is used. The backend reads the cookie directly.

## WebSocket Authentication

WebSocket connections cannot send cookies during the upgrade handshake in all browsers.

The frontend obtains a short-lived WebSocket token before connecting:

```
GET /api/ws-token   (Next.js route handler)
  → POST /api/ws/token   (backend)
```

The backend issues a WebSocket token stored in Redis under `ws_token:<token>` with a 90-second TTL.

The client connects using the token as a query parameter:

```
GET /ws?token=<ws_token>
```

`SessionExtract` detects the `?token=` query parameter and validates it against the `ws_token:` namespace instead of the `session:` namespace. WebSocket tokens are not slid — they expire after 90 seconds regardless of use.

## Logout and Revocation

Logout endpoint:

```
POST /api/auth/logout
```

The backend reads the `backend_session` cookie, deletes `session:<token>` from Redis, and returns 200. Errors are ignored so that logout always succeeds.

The frontend also deletes the `backend_session` cookie on signout.

Revocation takes effect immediately: once deleted from Redis, the token is rejected on the next request.

## Email Allowlist

An optional allowlist controls which email addresses may register.

The allowlist is checked **only during account creation** — when `ensure_user` creates a new user record for the first time. If the user already exists in the database, the allowlist is not consulted and login proceeds normally.

This means:

- Removing an email from the allowlist does not affect existing users.
- Existing users can always log in and their active sessions remain valid.
- The allowlist is a registration gate, not an ongoing access control mechanism.

`SessionExtract` does not re-check the allowlist.

If no allowlist is configured, all emails are permitted to register.

## Session Expiry and Renewal

Sessions use a sliding 24-hour TTL. Each authenticated request resets the expiry.

A session expires only if the user is inactive for 24 consecutive hours.

There is no client-side refresh. No JWT expiry checking is performed. The frontend does not need to pre-emptively renew sessions.

On session expiry, the next API call returns 401. The frontend redirects to sign-in.

## Error Responses

| Condition | Status | Code |
|---|---|---|
| Cookie absent | 401 | `UNAUTHORIZED` |
| Token not found in Redis | 401 | `UNAUTHORIZED_INVALID_TOKEN` |
| Redis unavailable | 503 | `REDIS_UNAVAILABLE` |
| Email not in allowlist | 403 | `EMAIL_NOT_ALLOWED` |

## Testing

Tests that exercise `SessionExtract` end-to-end require a real Redis connection (`REDIS_URL`). Tests that only test route handlers or downstream logic use `TestSessionInjector`, which injects `SessionData` directly into request extensions without Redis.
