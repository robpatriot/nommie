# Testing Rules

- Tests must be deterministic.
- Add tests for complex logic and edge/error paths.
- Prefer structured assertions over string matching.
- Use repository scripts in package.json as canonical for running tests, lint, and build.
- If behavior changes, ensure relevant test coverage exists or is updated.

## Deterministic Time and Scheduling

### Requirement

Tests must not depend on wall-clock time.

Any application logic that uses timers for **application control flow**
(polling, retries, backoff, reconnect loops, readiness checks, or similar
coordination logic) must be implemented so that tests can deterministically
control scheduling.

Tests must be able to advance timer-driven behavior explicitly rather than
waiting for real time to pass.

This requirement applies only when the timer-driven logic is part of the
application’s behavior that must be tested deterministically.

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

### Required Architecture for Timer-Driven Logic

When application control-flow logic requires scheduling, it must use an
injectable scheduler or driver abstraction rather than directly calling
`setTimeout` or `setInterval`.

The abstraction must:

- Provide a method for scheduling delayed work.
- Return a handle that allows scheduled work to be cancelled.
- Allow tests to control execution deterministically.

Production code must use a scheduler implementation backed by real timers.

Tests must use a manual driver implementation that:

- Stores scheduled callbacks in memory
- Executes them only when explicitly triggered by the test
- Provides a deterministic mechanism for advancing scheduled work
  (for example a `tick()` method)

---

### React Testing Requirements

If advancing the scheduler triggers React state updates, the scheduler
advancement must occur inside `act(...)`.

Typical pattern:

`await act(async () => { await driver.tick() })`

Tests must not rely on `waitFor` to advance timers.

`waitFor` may only be used for non-timer asynchronous settling and should use
short default timeouts.

---

### When This Rule Applies

This deterministic scheduling requirement applies to control-flow timers used
for application coordination, including:

- polling loops
- retry logic
- exponential backoff
- WebSocket reconnect timers
- synchronization or readiness polling
- coordination logic based on scheduled checks

---

### When This Rule Does Not Apply

Agents must **not** introduce scheduler abstractions for timers used only for
cosmetic UI behavior or presentation concerns, including:

- animations
- temporary visual delays
- toast auto-dismiss timers
- minor UI debouncing
- performance logging or diagnostics delays

Such timers may continue to use normal browser scheduling.

Agents must also **not** refactor third-party integrations or external library
code solely to satisfy this rule.
