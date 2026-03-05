# AI Player Interface

## Purpose

Defines the integration contract for production AI players.

This document specifies:

- how AI implementations are constructed and registered
- the call contract for decisions
- determinism expectations for tests
- constraints required for correctness and safety

## Rules

AI behavior must conform to the canonical ruleset in `game-rules.md`.

## AI Contract

An AI implements an interface with three decision methods:

- choose_bid
- choose_trump
- choose_play

The engine calls each method only during the corresponding phase.

An AI must always return a legal result or a structured error.

The AI must never panic.

## Legal Move Source of Truth

Legality is defined by engine helpers.

AI implementations must not re-implement legality rules.

- bids: use the engine-provided legal bid helper (includes dealer restriction and zero-bid streak rule)
- trumps: use the engine-provided legal trump helper
- plays: use the engine-provided legal play helper (includes follow-suit enforcement)

## Determinism

Deterministic test execution must be possible.

Rules:

- AI constructors must accept an optional seed.
- when seeded, AI decisions must be reproducible for identical inputs.
- production may use entropy when no seed is provided.

The game engine separately enforces determinism for dealing and memory degradation from game-level seeds.

AI implementations must not rely on global RNG state.

## Concurrency and State

AI implementations must be safe for concurrent execution.

Rules:

- AI values must be Send + Sync.
- do not store per-game mutable state inside the AI.
- if internal RNG is maintained, it must be synchronized.

The engine may create new AI instances per decision.

## Registration

Production AIs are exposed through a static registry.

Rules:

- each AI has a stable name and semantic version
- registry ordering is append-only unless a breaking change is intended
- constructors must be side-effect free
- constructors must respect the optional seed input

## Error Handling

AI errors are structured.

Rules:

- use an invalid-move error only when no legal decision can be produced
- internal errors must not leak sensitive data
- illegal moves and AI errors are treated as failures by the engine

Engine behavior on repeated AI failures is implementation-defined and should be treated as a hard failure mode for the game.

## Testing Requirements

Each AI must pass a deterministic conformance suite.

Tests must validate:

- bids returned are legal (including dealer restriction and zero-bid streak rule)
- trump returned is legal for the phase and caller
- plays returned are legal and follow suit when required
- seeded determinism for identical inputs

