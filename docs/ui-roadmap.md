## Nommie UI Roadmap

This document is the canonical, living plan for building the Nommie UI on web (Next.js) and, later, a mobile client. It captures stages, decisions, endpoints, and a lightweight progress log so work can continue seamlessly across machines.

### Stack (confirmed)
- **Web**: Next.js App Router (`apps/frontend`), server components + server actions, Tailwind CSS
- **Auth**: NextAuth (Google) on web; backend issues JWT used via `fetchWithAuth`
- **Backend**: Rust service with DB; REST endpoints consumed from web using `NEXT_PUBLIC_BACKEND_BASE_URL`

### Product scope (initial)
- Multiplayer Nomination Whist ("Nommie")
- Flows: landing → auth → lobby → game room → in-game table (bidding, trick play, scoring) → summary

---

## Staged Plan

Each stage has learning goals, deliverables, and a concise definition of done (DoD).

### Stage 0 — Align and prepare
- Learning goals: how web talks to backend; server vs client components
- Deliverables:
  - Simple wireframes for Lobby and Game Room
  - List of game phases and data needed per screen
  - Endpoint shortlist for MVP
- DoD: one-page sketch + endpoint list committed alongside this doc

### Stage 1 — App shell and routing (web)
- Learning goals: Next.js App Router, protected pages, basic navigation
- Deliverables:
  - Routes: `/lobby` and `/game/[gameId]` (skeletons)
  - `Header` shows auth state; link to Lobby when signed in
- DoD: Navigate `/` → `/lobby` → `/game/[gameId]` with placeholder content

### Stage 2 — Read-only lobby
- Learning goals: server data fetching, loading/empty/error states, manual refresh
- Deliverables:
  - List joinable games from backend
  - Empty state + loading skeleton + error banner + refresh button
- DoD: Stable read-only game list, verifiable with backend data

### Stage 3 — Create and join game
- Learning goals: server actions vs API calls; form handling; error surfacing
- Deliverables:
  - Create Game modal (minimal inputs)
  - Join action navigating to `/game/[gameId]`
  - Toaster + error details (include `traceId` when present)
- DoD: Create → redirect to game page; join from lobby works

### Stage 4 — Read-only game room and table snapshot
- Learning goals: map domain to UI components; phased rendering
- Deliverables:
  - Render phase, seats/players, hand snapshot (no interactions)
  - Trick area, bids/score panel placeholders; turn indicator
  - Light polling (manual or timer), with room for ETag optimization later
- DoD: Page reflects backend state changes without interactions

### Stage 5 — Core interactions: ready, start, bid, play
- Learning goals: pessimistic writes, legal moves gating, robust error handling
- Deliverables:
  - Ready/unready; host start
  - Bid panel; play a card from legal subset
  - Disable illegal moves, still handle server rejection
- DoD: Two browser sessions can play through a round

### Stage 6 — UX fit and accessibility
- Learning goals: keyboard-first play, ARIA, motion for feedback
- Deliverables:
  - Keyboard selection/submit for cards
  - Focus management, labels, color contrast
  - Subtle animations for plays and trick wins
- DoD: Fully keyboard-operable play; a11y checks pass basic audit

### Stage 7 — Mobile foundations (Expo)
- Learning goals: React Native/Expo basics; shared types and API
- Deliverables:
  - `apps/mobile` (Expo) scaffold
  - Shared `packages/shared` for types and API wrapper
  - Read-only lobby list screen
- DoD: Lobby list works on emulator/device against same backend

### Stage 8 — Mobile game: read-only then interactions
- Learning goals: mobile layouts/gestures; simplified table UI
- Deliverables:
  - Render simplified game snapshot; then bid/play with pessimistic writes
- DoD: Two devices can join, bid, and play a trick

---

## Endpoints (initial target set)
Note: Align with backend routes; adjust names/paths as needed.
- Auth/session: frontend uses NextAuth; backend JWT via `auth()` in server context
- Games:
  - GET list joinable games
  - POST create game
  - POST join game
  - POST ready/unready
  - POST start game (host only)
- In-game actions and views:
  - GET player view / game snapshot (phase, hands, legal actions, trick state)
  - POST bid
  - POST play
  - GET scores/history

Error model: Use `BackendApiError` on the frontend; surface `traceId` when present.

---

## Decisions
- Transport: Start with polling (manual → simple interval). Keep update orchestration isolated to swap in SSE/WS later.
- Caching: Keep minimal at first; introduce TanStack Query only if complexity grows.
- Optimism: Pessimistic writes for moves/bids; optimistic only for non-critical toggles.
- Accessibility: Keyboard play is a first-class requirement, not a later add-on.

## Open Questions
- Spectators supported?
- AI seat controls (host-only?) and visibility rules
- Rejoin semantics after disconnect
- Game creation rule toggles for MVP

---

## Progress Tracker
Use checkboxes to mark completion. Add brief notes/dates.

- [ ] Stage 0 — Align and prepare
  - [ ] Wireframes committed (screenshots or links)  
  - [ ] Endpoint shortlist verified with backend
- [ ] Stage 1 — App shell and routing (web)
  - [ ] `/lobby` placeholder
  - [ ] `/game/[gameId]` placeholder
- [ ] Stage 2 — Read-only lobby
- [ ] Stage 3 — Create and join game
- [ ] Stage 4 — Read-only game room and table snapshot
- [ ] Stage 5 — Core interactions
- [ ] Stage 6 — UX and accessibility
- [ ] Stage 7 — Mobile foundations (Expo)
- [ ] Stage 8 — Mobile interactions

---

## Working Notes
- Environment: ensure `NEXT_PUBLIC_BACKEND_BASE_URL` is set locally; keep `.env` out of git; maintain `.env.example` with placeholders.
- Data sync: Prefer manual refresh initially; add interval polling with conservative cadence (lobby slower, active turn faster).
- Testing: Start with a pair of browser tabs; later, add a small E2E for create→join→bid→play→score.

---

## Change Log (most recent first)
- YYYY-MM-DD: Created initial roadmap with staged plan and tracker.


---

## Stage 0 – Wireframing Guide (temporary scaffolding)

This section is a step-by-step helper to create first wireframes. We will replace it with your own notes once you draft them.

### Step 1: Pick your tool (1–2 minutes)
- Pen and paper, or
- Text-only description to paste below, or
- Simple box tool (Excalidraw/FigJam), optional.

### Step 2: Wireframe the Lobby (5–10 minutes)
Describe sections top-to-bottom and fill the template.

- Header:
  - Left: app name
  - Right: auth status + Sign in/out
- Main area:
  - Toolbar: Create Game, Refresh
  - Game list with columns: Name | Players x/y | Status | Actions [Join]
  - Empty state: “No games yet. Create one.”
- Footer: small help text

Template to fill:
- Layout: single column, centered max-width
- Toolbar: [Create Game], [Refresh]
- Table columns: Name | Players “x/y” | Status (open/in-progress) | Actions [Join]
- Empty state message: …
- Assumptions:
  - Max players per game: ?
  - Show private games or only public? 
  - Can you join in-progress games?
- Notes/variants: e.g., pagination needed or simple “Load more”

Example (editable):
- Layout: Centered page. Toolbar with “Create Game” and “Refresh”.
- Game list: 3–10 items. Each row has a Join button.
- Status chips: Open (green), In‑progress (yellow), Full (gray).
- Assumptions: Max players 4. Only open games listed. No pagination yet.

### Step 3: Wireframe the Game Room (10–15 minutes)
Sections:

- Header: Game name/ID, Back to Lobby, your auth status
- Top row: Phase indicator (Dealing/Bidding/Trick/Scoring) and Turn badge
- Table area (center): Trick area; opponents around with name/bid/tricks
- Bottom: Your hand (card fan) and Action panel
  - If Bidding: bid selector (0..N) + submit
  - If Trick: play card (only legal cards enabled)
- Sidebar (right or collapsible): Scoreboard, Ready/Unready, Start (host only), Copy invite link

Template to fill:
- Layout: table centered; sidebar right (collapsible on mobile)
- Phase: [Bidding | Trick | Scoring], Turn: [PlayerName]
- Seats: Self bottom; others left/top/right with name + bid + tricks
- Trick area: up to 4 cards in play (positions match seats)
- Your hand: 8–13 cards; legal cards enabled; others disabled
- Actions: Bid dropdown + Submit OR Play button on each card
- Assumptions:
  - Max players: ?
  - Are spectators shown?
  - Host-only Start button? Add/remove AI?
- Notes: keyboard controls later; animations later

Example (editable):
- Layout: Center table; sidebar with scores and controls.
- Phase: Bidding; Turn: You.
- Seats: 4 players; badges show bids/tricks.
- Hand: 8 cards; select one; disabled if illegal.
- Actions: Bid dropdown 0–8; Submit; Ready/Unready in sidebar.
- Assumptions: Host can start; no spectators for MVP.

### Step 4: Annotate assumptions directly
- Add an “Assumptions” list under each screen.
- Mark any that change layout or API with [BLOCKING].

### Step 5: Extract 3–5 blocking questions
Pick only those that affect Stage 1–3 layout or API contracts, e.g.:
- Do we list in‑progress games in Lobby? [affects columns/actions]
- Max players per game? [affects seat layout]
- Can non-host start the game? [affects visibility of Start control]

### Step 6: Paste your text below (temporary)
Paste your Lobby and Game Room descriptions here. We’ll integrate them above and delete this helper section afterwards.


