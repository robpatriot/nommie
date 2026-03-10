# Testing Rules

This document defines how tests must be written and executed in this repository.

Agents must follow these rules whenever adding, modifying, or validating code.

---

# Core Principles

- Tests must be **deterministic**.
- Add tests for **complex logic, edge cases, and error paths**.
- Prefer **structured assertions** over string matching.
- If behaviour changes, ensure **relevant tests are added or updated**.
- Follow **existing testing patterns in the repository**.

Agents must not introduce new testing styles when established patterns already exist.

---

# Running Tests

Tests must be executed using the **repository scripts defined in `package.json`**.

These scripts are the **canonical interface** for running tests, linting, and builds.

Agents must **not invoke language-native test runners directly**.

Prohibited examples:

- `cargo test`
- `vitest`
- `jest`
- any direct test runner invocation

Always run the appropriate script from `package.json`.

If uncertain which script to use, inspect `package.json` and follow the existing conventions.

---

# Test Execution Requirement

Testing is a **required completion step for every task**.

After making any change that could affect behaviour, agents must:

1. Run the relevant repository test script(s)
2. Confirm tests pass
3. Fix failures if they occur

A task is **not complete until tests pass**.

---

# Deterministic Async and Time Behaviour

Tests must **not depend on real time passing**.

Agents must not introduce:

- sleeps or delays (`setTimeout`, `sleep`, `delay`)
- fake timer frameworks (`vi.useFakeTimers`, `jest.useFakeTimers`)
- timer advancement helpers (`advanceTimersByTime`, `runAllTimers`)

Instead, follow the **deterministic patterns already used in the repository**:

- **Scheduler/driver pattern** for timer-driven logic (polling, retries, reconnect loops)
- **Controlled promises or deferred resolution** for non-timer async behaviour

Tests must control when asynchronous behaviour progresses rather than waiting for time.

---

# React Testing

If advancing a scheduler or resolving async work triggers React state updates,
the operation must occur inside `act(...)`.

Example pattern:

await act(async () => {
  await driver.tick()
})

`waitFor` must not be used to simulate timer progression.

---

# Scope of These Rules

Do not introduce scheduler abstractions for cosmetic UI timers such as:

- animations
- temporary visual delays
- toast auto-dismiss timers
- minor UI debouncing

Also do not refactor third-party code solely to satisfy these testing rules.
