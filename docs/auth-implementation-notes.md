# Auth Migration — Implementation Notes

Non-obvious gotchas from a previous implementation attempt. These are things that won't be apparent from reading the codebase or `docs/auth.md`.

## Rust compiler traps

**E0373 in test closures.** `.with_routes(|cfg| { ... })` requires the closure to be `'static`. Any captured `String` must be cloned before the closure, and the closure must be `move`:

```rust
let sub = sub.clone();
let email = email.clone();
app_builder.with_routes(move |cfg| {
    cfg.service(
        web::scope("/x")
            .wrap(TestSessionInjector::new(user_id, sub.clone(), email.clone()))
            ...
    );
})
```

**Service trait must be in scope.** Calling `.call(req)` on a service directly requires `use actix_web::dev::Service;` — otherwise the method simply isn't found and the error is misleading.

**`&str` is not `std::error::Error`.** Some `AppError` constructors take a source error. Passing a string literal won't compile. There is a `Sentinel("message")` newtype in the codebase that implements `std::error::Error` for exactly this case.

## Test architecture

**Most tests should not need Redis.** Only tests exercising `SessionExtract` end-to-end need a real Redis connection. Route and handler tests should use a `TestSessionInjector` middleware that injects `SessionData` directly into request extensions, bypassing Redis entirely.

**WebSocket test ordering.** `create_test_ws_token(&state, ...)` must be called **before** `start_test_server(state, ...)` because `start_test_server` takes ownership of `state`. The token remains valid because the Redis connection is `Arc`-backed — the running server shares the same connection.

## Allowlist enforcement

The allowlist is a registration gate only — it must be checked solely during account creation (`ensure_user`), not at login for existing users and not on every request in `SessionExtract`.

The current `auth.ts` NextAuth `signIn` callback hits `/api/auth/check-allowlist` before creating a NextAuth session. This should be removed — NextAuth has no way to know whether the user is new at that point, so the pre-check incorrectly blocks returning users who were later removed from the allowlist. The backend `ensure_user` is the single enforcement point.

`SessionExtract` must not re-check the allowlist. Removing a user from the allowlist must not invalidate their active sessions.

## Token format

`generate_session_token()` returns a UUID v4 in simple format: **32 lowercase hex characters**. Any assertion on token length should use 32, not 64.
