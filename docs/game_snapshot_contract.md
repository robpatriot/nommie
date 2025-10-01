# Game Snapshot (Wire Contract)

**Purpose:**
Canonical, read-only view of a game for the frontend. This is the single source of truth for rendering and client-side logic.

---

## Root Shape

- **`GameHeader`**: Immutable identifiers & top-level metadata
  (e.g., game id, players, seat order, current dealer if relevant).

- **`PhaseSnapshot`** (discriminated union): exactly one active phase at any time:
  - `Init`
  - `Bidding`
  - `TrumpSelect` (may include `NO_TRUMP`)
  - `Trick`
  - `Scoring`
  - `Complete`

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
- **Trump semantics:** `Trump` can be any suit or `NO_TRUMP`.
  - When `NO_TRUMP`, trick winners are decided purely by lead suit
    (highest of lead suit wins if no trumps are present).
- **Follow-suit rule:**
  - If a player can follow the lead suit, they must.
  - FE should **not** replicate legality checks; treat snapshot as truth.
- **Player seats:**
  - Seats are stable identifiers (e.g., N/E/S/W or indexed).
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
- (We may add an explicit `version` field later.)

---

## Error Handling

- The snapshot route returns a structured error (`AppError`) for not-found/invalid state.
- FE should parse and present these gracefully.

---

## Testing References

- Golden JSON fixtures (coming soon) will illustrate representative states:
  - `Init`
  - `Bidding`
  - `TrumpSelect (NO_TRUMP)`
  - `Trick`
  - `Scoring`
  - `Complete`
- FE can use these as examples for parsing and UI snapshots.

---

## High-Level Examples

### `GameHeader`
{
  "game_id": "uuid-here",
  "players": [
    { "seat": "N", "user_id": "u1", "display_name": "Alice" },
    { "seat": "E", "user_id": "u2", "display_name": "Bob" }
  ],
  "dealer": "W"
}

### `RoundPublic`
{
  "round_no": 3,
  "hand_size": 11,
  "leader_seat": "S",
  "trick_no": 2
}

### `PhaseSnapshot` Examples

- **Bidding**
{
  "phase": "Bidding",
  "bids": [{ "seat": "N", "tricks": 4 }],
  "current_seat": "E"
}

- **TrumpSelect**
{
  "phase": "TrumpSelect",
  "winner_seat": "N",
  "trump": "NO_TRUMP"
}

- **Trick**
{
  "phase": "Trick",
  "plays": [
    { "seat": "N", "card": "AS" },
    { "seat": "E", "card": "KH" }
  ],
  "lead_suit": "Spades",
  "to_play": "S"
}

- **Scoring**
{
  "phase": "Scoring",
  "tally": [
    { "seat": "N", "tricks": 4, "bonus": true },
    { "seat": "E", "tricks": 3, "bonus": false }
  ]
}

- **Complete**
{
  "phase": "Complete",
  "final_scores": [
    { "seat": "N", "points": 42 },
    { "seat": "E", "points": 35 }
  ]
}

