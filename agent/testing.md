# Testing Rules

- Tests must be deterministic.
- Add tests for complex logic and edge/error paths.
- Prefer structured assertions over string matching.
- Use repository scripts in package.json as canonical for running tests, lint, and build.
- If behavior changes, ensure relevant test coverage exists or is updated.

## Deterministic Time and Async Control

### Requirement

Tests must not depend on wall-clock time.

Tests must complete without waiting for real time to pass. Any behavior that
would otherwise depend on timers, delays, or deferred async completion must be
controlled deterministically by the test.

There are two required patterns:

- **Deterministic scheduler control** for timer-driven application control flow
- **Controlled promises / deferred async** for non-timer asynchronous lifecycle

Agents must choose the pattern that matches the behavior under test.

---

### Prohibited Test Patterns

Agents must not introduce or rely on the following patterns:

- Global fake timers (`vi.useFakeTimers`, `jest.useFakeTimers`)
- Timer advancement hacks (`advanceTimersByTime`, `runAllTimers`,
  `runOnlyPendingTimers`)
- Sleep-based waiting (`new Promise(resolve => setTimeout(...))`,
  `sleep()`, `delay()`)
- Long `waitFor` timeouts used to simulate timer progression
- Test-only reductions of polling intervals or retry delays
- Any approach that depends on real time passing

These approaches introduce nondeterminism, slow test suites, and CI flakiness.

---

### Required Pattern: Timer-Driven Control Flow

When application control-flow logic requires scheduling, it must use an
injectable scheduler or driver abstraction rather than directly calling
`setTimeout` or `setInterval`.

This applies to behavior where scheduled progression is part of the logic being
tested, including:

- polling loops
- retry logic
- exponential backoff
- WebSocket reconnect timers
- readiness checks
- other scheduled coordination logic

The abstraction must:

- Provide a method for scheduling delayed work
- Return a handle that allows scheduled work to be cancelled
- Allow tests to control execution deterministically

Production code must use a scheduler implementation backed by real timers.

Tests must use a manual driver implementation that:

- Stores scheduled callbacks in memory
- Executes them only when explicitly triggered by the test
- Provides a deterministic mechanism for advancing scheduled work

Tests must advance scheduler-driven behavior explicitly. They must not wait for
time to pass.

---

### Required Pattern: Non-Timer Async Lifecycle

If the behavior under test is asynchronous but not truly timer-driven, tests
must use controlled async primitives rather than delays.

This applies to cases such as:

- pending network requests
- mutation in-flight state
- success or failure resolution
- rollback behavior
- async loading transitions
- coordination around promise completion

Tests should use controlled promises, deferred resolution, or equivalent
explicit mechanisms so the test decides when async work resolves or rejects.

Agents must not simulate these states by inserting sleeps or mock timers.

Agents must not introduce scheduler abstractions for these cases unless the
production behavior is genuinely driven by scheduled control flow.

---

### React Testing Requirements

If advancing a scheduler triggers React state updates, scheduler advancement
must occur inside `act(...)`.

Typical pattern:

`await act(async () => { await driver.tick() })`

If resolving or rejecting a controlled promise triggers React state updates,
that transition must also be observed safely through `act(...)` or normal
Testing Library async utilities.

`waitFor` must not be used to simulate timer progression.

`waitFor` may be used only for non-timer asynchronous settling and should rely
on short default timeouts.

---

### When This Rule Does Not Apply

Agents must **not** introduce scheduler abstractions for timers used only for
cosmetic UI behavior or presentation concerns, including:

- animations
- temporary visual delays
- toast auto-dismiss timers
- minor UI debouncing
- performance logging or diagnostics delays

Such timers may continue to use normal browser scheduling unless there is a
separate explicit reason to refactor them.

Agents must also **not** refactor third-party integrations or external library
code solely to satisfy this rule.
