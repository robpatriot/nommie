# Game Snapshot (Wire Contract)

## Document Scope

Defines the serialized shape delivered to clients for rendering active games.
Pair this with `dev-roadmap.md` for UX expectations and
`architecture-game-context.md` for the backend assembly pipeline.

**Purpose:**
Canonical, read-only view of a game for the frontend. This is the single source of truth for rendering and client-side logic.

---

## Root Shape

- **`GameHeader`**: Immutable identifiers & top-level metadata
  (e.g., game id, players, seat order, current dealer if relevant).

- **`PhaseSnapshot`** (discriminated union): exactly one active phase at any time:
  - `Init`
  - `Bidding`
  - `TrumpSelect` (may include `NO_TRUMPS`)
  - `Trick`
  - `Scoring`
  - `Complete`
  - `GameOver`

- **`RoundPublic`**: Public round facts
  (round number, hand size, leader seat, trick number, etc.).

---

## Discriminated Union

- The phase is tagged (serde adjacently or internally tagged).
- FE should **switch on this tag** to branch rendering/logic.
- Only fields for the active phase will be present.
- Do **not** rely on inactive-phase fields.

---

## Conventions

- **Trick numbering:** `trick_no` is **1-based**. The first trick in a round has `trick_no = 1`.
- **Trump semantics:** `Trump` can be any suit or `NO_TRUMPS`.
  - When `NO_TRUMPS`, trick winners are decided purely by lead suit
    (highest of lead suit wins if no trumps are present).
- **Follow-suit rule:**
  - If a player can follow the lead suit, they must.
  - FE should **not** replicate legality checks; treat snapshot as truth.
- **Player seats:**
  - Seats are numeric identifiers (0, 1, 2, 3) representing the four player positions.
  - Rotation/next-to-play is derivable from snapshot data.
  - Future helpers may be provided server-side.
- **Public vs private info:**
  - Snapshot exposes **public** state only.
  - Hidden information (e.g., full hands) is not included unless explicitly allowed.

---

## Enums & Casing

- Enum casing and string values are **stable** once published.
- Only **new** enum variants may be added in non-breaking expansions.
- Removals/renames are considered breaking changes and will be coordinated.

---

## Stability & Evolution

- Snapshot shape is intended to be stable during active FE development.
- Additions may occur in non-breaking ways (new optional fields / new tagged variants).
- Breaking changes will be called out in PRs and release notes.
- A `version` field may be added in a future update to support explicit contract versioning.

---

## Error Handling

- The snapshot route returns a structured error (`AppError`) for not-found/invalid state.
- FE should parse and present these gracefully.

---

## Testing References

- Golden JSON fixtures illustrate representative states:
  - `Init`
  - `Bidding`
  - `TrumpSelect (NO_TRUMPS)`
  - `Trick`
  - `Scoring`
  - `Complete`
- FE can use these as examples for parsing and UI snapshots.

---

## High-Level Examples

### `GameHeader`
```json
{
  "round_no": 3,
  "dealer": 0,
  "seating": [
    { "seat": 0, "user_id": 1, "display_name": "Alice", "is_ai": false, "is_ready": true },
    { "seat": 1, "user_id": 2, "display_name": "Bob", "is_ai": false, "is_ready": true }
  ],
  "scores_total": [10, 8, 12, 9],
  "host_seat": 0
}
```

### `RoundPublic`
```json
{
  "hand_size": 11,
  "leader": 2,
  "bid_winner": 0,
  "trump": "Spades",
  "tricks_won": [3, 2, 4, 2]
}
```

### `PhaseSnapshot` Examples

- **Bidding**
```json
{
  "phase": "Bidding",
  "data": {
    "round": { "hand_size": 11, "leader": 0, "bid_winner": null, "trump": null, "tricks_won": [0, 0, 0, 0] },
    "to_act": 1,
    "bids": [4, null, null, null],
    "min_bid": 0,
    "max_bid": 11
  }
}
```

- **TrumpSelect**
```json
{
  "phase": "TrumpSelect",
  "data": {
    "round": { "hand_size": 11, "leader": 0, "bid_winner": 0, "trump": null, "tricks_won": [0, 0, 0, 0] },
    "to_act": 0,
    "allowed_trumps": ["Clubs", "Diamonds", "Hearts", "Spades", "NO_TRUMPS"]
  }
}
```

- **Trick**
```json
{
  "phase": "Trick",
  "data": {
    "round": { "hand_size": 11, "leader": 0, "bid_winner": 0, "trump": "Spades", "tricks_won": [3, 2, 2, 2] },
    "trick_no": 2,
    "leader": 1,
    "current_trick": [
      [0, "AS"],
      [1, "KH"]
    ],
    "to_act": 2,
    "playable": ["2C", "3C", "5C"]
  }
}
```

- **Scoring**
```json
{
  "phase": "Scoring",
  "data": {
    "round": { "hand_size": 11, "leader": 0, "bid_winner": 0, "trump": "Spades", "tricks_won": [4, 3, 2, 2] },
    "round_scores": [14, 3, 2, 2]
  }
}
```

- **Complete**
```json
{
  "phase": "Complete",
  "data": {
    "round": { "hand_size": 1, "leader": 0, "bid_winner": 0, "trump": "Spades", "tricks_won": [1, 0, 0, 0] }
  }
}
```
