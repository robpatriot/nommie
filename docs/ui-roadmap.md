## Nommie UI Roadmap

This document is the canonical, living plan for building the Nommie UI on web (Next.js) and, later, a mobile client. It captures stages, decisions, endpoints, and a lightweight progress log so work can continue seamlessly across machines.

### Stack (confirmed)
- **Web**: Next.js App Router (`apps/frontend`), server components + server actions, Tailwind CSS
- **Auth**: NextAuth (Google) on web; backend JWT stored server-only in NextAuth JWT token (never exposed to client), resolved via `resolveBackendJwt()` and `requireBackendJwt()` helpers in server components/actions; proactive refresh within 5 minutes of expiry
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
  - Root route `/`: Welcome page (login button) for non-authenticated; redirect to `/lobby` for authenticated
  - Routes: `/lobby` and `/game/[gameId]` (skeletons)
  - `Header` shows auth state; Google sign-in required; link to Lobby when signed in; "Resume last game" CTA (via backend `last-active`)
- DoD: Navigate `/` → `/lobby` → `/game/[gameId]` with placeholder content

### Stage 2 — Read-only lobby
- Learning goals: server data fetching, loading/empty/error states, manual refresh
- Deliverables:
  - Two lists:
    - Joinable games (not started yet). If user is already in a game, hide/disable create/join.
    - In‑progress games (others; view‑only, informational)
  - "Resume last game" CTA at top (also exposed in Header)
  - Client-side search/filter (MVP; can upgrade to backend later)
  - Empty state + loading skeleton + error banner + refresh button
- DoD: Stable read-only game list, verifiable with backend data

### Stage 3 — Create and join game
- Learning goals: server actions vs API calls; form handling; error surfacing
- Deliverables:
  - Create Game modal (minimal inputs):
    - Name (optional; defaults to "{CreatorName} game" if blank; uniqueness not required)
    - Starting dealer (optional; defaults to game creator)
  - Join action navigating to `/game/[gameId]`
  - Toaster + expandable error details; `traceId` hidden until expanded; log `traceId` in dev
- DoD: Create → redirect to game page; join from lobby works

### Stage 4 — Read-only game room and table snapshot
- Learning goals: map domain to UI components; phased rendering
- Deliverables:
  - Render phase, seats/players, hand snapshot (no interactions)
  - Trick area, bids/score panel placeholders; turn indicator
  - Collapsible score table in sidebar (cumulative totals; always accessible)
  - Light polling (manual or timer), with room for ETag optimization later
    - Cadence: Consistent 3-second polling regardless of turn/phase
  - Subtle polling indicator near Phase/Turn (e.g., syncing dot with tooltip/`aria-live="off"`)
- DoD: Page reflects backend state changes without interactions

### Stage 5 — Core interactions: ready, start, bid, play
- Learning goals: pessimistic writes, legal moves gating, robust error handling
- Deliverables:
  - Ready (no unready); AI auto-ready; auto-start when all players are ready
  - Bid panel; play a card from legal subset
  - Disable illegal moves, still handle server rejection
  - Host-only: add/remove AI seats before start (clearly labeled), up to 4 total players
- DoD: Two browser sessions can play through a round

### Stage 6 — UX fit and accessibility
- Learning goals: keyboard-first play, ARIA, motion for feedback
- Deliverables:
  - Keyboard selection/submit for cards
  - Focus management, labels, color contrast
  - Subtle animations for plays and trick wins
- DoD: Fully keyboard-operable play; a11y checks pass basic audit

Detailed UX spec (MVP)
- Lobby
  - Layout: Single column, centered, max-width ~1100px. Sticky header with app name (left) and auth/avatar (right).
  - Toolbar: [Create Game] primary, [Refresh] secondary, search input, status chips filter.
  - Table: Name | Players x/y | Status | Actions [Join/Full]. Semantic <table>, visible focus, 4.5:1 contrast.
  - Empty state: “No games yet. Create one.” with inline [Create Game]. Optional row expand for metadata.
  - Keyboard: Tab → toolbar → header → first row; Arrow Up/Down navigate rows; Enter joins; `r` refresh; `c` create; `f` focus search.
  - Mobile: Stacked card rows; actions remain accessible on the right.
- Game Room
  - Layout: Center table with seats Bottom(You)/Left/Top/Right; collapsible right sidebar (scores/controls).
  - Header: Game name/ID, Back to Lobby, auth/avatar; Copy Invite Link in overflow.
  - Phase/Turn: Top row announces Phase (Bidding/Trump Selection/Trick/Scoring) and Turn badge.
  - Trick area: Up to 4 cards placed by seat; subtle entry animation; last trick summary chip.
  - Seats: Name, bid (during Bidding), tricks won; active turn ring.
  - Hand: Fan/grid; illegal cards disabled and not focusable; legal cards navigable.
  - Actions: Bidding (0..hand size selector + Submit). Trick (select card then Play, or Enter on focused card).
  - Keyboard: `?` shortcuts; `g l` Lobby; `s` Sidebar; Hand (Arrows/Home/End/Enter/Esc); Bidding (number keys/Up/Down/Enter/Esc); Ready `y`; Start `Shift+S` (host confirm).
  - A11y: Live region for phase/turn; dialogs trap focus; cards `aria-label` (e.g., “Seven of Hearts, legal”); honors `prefers-reduced-motion`.
  - Motion: 150–200ms ease-out for play/trick-win; subtle elevation/pulse for active turn.
  - Errors: Toast summary; details panel with server message and `traceId` link.

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
- Auth/session: frontend uses NextAuth; backend JWT stored server-only in NextAuth JWT token, resolved via `resolveBackendJwt()` and enforced with `requireBackendJwt()` in server components/actions; proactive refresh within 5 minutes of expiry
- Games:
  - GET list joinable games
  - GET list in-progress games (view-only, informational)
  - POST create game
  - POST join game
  - POST ready (MVP: no unready)
  - POST start game (auto when all ready; explicit host action if needed for edge cases)
  - POST add AI seat (host only)
  - POST remove AI seat (host only)
  - GET last-active game for current user (for Resume CTA)
- In-game actions and views:
  - GET player view / game snapshot (phase, hands, legal actions, trick state)
  - POST bid
  - POST select trump (highest bidder only, after all bids)
  - POST play
  - GET scores/history

Error model: Use `BackendApiError` on the frontend; `traceId` available in details (hidden until expanded); log `traceId` in dev.

---

## Decisions
- Transport: Start with polling (manual → simple interval). Keep update orchestration isolated to swap in SSE/WS later.
- Caching: Keep minimal at first; introduce TanStack Query only if complexity grows.
- Optimism: Pessimistic writes for moves/bids; optimistic only for non-critical toggles.
- Accessibility: Keyboard play is a first-class requirement, not a later add-on.
 - Layout: Centered container with desktop max-width (~1100px); fluid below that; full-bleed only where it adds value.
 - Lobby lists: Show joinable games separately from an In‑progress (view‑only) list.
 - Player count: Exactly 4 players per game (min=max); cannot join once in progress.
 - Spectators: Deferred (no spectator mode in MVP). In‑progress list is informational only.
 - Rejoin: A player can always reclaim their seat for the duration of the game; resume from Lobby "Your games" list.
 - Game creation (MVP): Only two options — optional name (with default) and starting dealer (defaults to creator). No other rule toggles.
 - Active game limit (MVP UX): One active game per player; UI hides/disables create/join when already in a game.
 - Resume CTA: Provide "Resume last game" both in Header (signed‑in) and on Lobby top.
  - Resume source: Backend-driven `GET /games/last-active` with client fallback to local recent only if endpoint fails.
 - Auth (MVP): Google via NextAuth is required; no guest/anonymous mode.
 - Error UX: Friendly summary in toast; `traceId` shown only in expanded details; log `traceId` in dev console.
 - AI seats (MVP): Included for manual testing; host-only add/remove; bots clearly labeled.
 - Minimum humans to start: 1 (creator). Remaining seats can be filled with AI.
 - Bid rules (MVP): Nil (0) bids allowed; maximum bid equals current hand size.
 - Trump: Selected by highest bidder after all bids; choice visible to all immediately; supports No Trumps.
 - Dealer/lead: Dealer rotates clockwise each round; player left of dealer leads first trick.
 - Ready/start (MVP): Players can only ready (no unready); AI auto-ready; game auto-starts when all are ready.
 - Game naming: Default to "{CreatorName} game"; names need not be unique (one active game per user UX).
 - Invites (MVP): Copy invite link only (full URL for direct browser paste); no visible table ID/share code.
 - Lobby sorting: Joinable list sorted by most players waiting (descending).
 - Illegal move errors: Server rejection surfaced via toast only.
 - Sync UX: No explicit "Sync now" in Game Room; show subtle polling indicator for confidence. Consistent 3-second polling in Stage 4 (read-only); can optimize cadence in Stage 5 based on interactions.
 - Scoring UI: Collapsible score table always visible in sidebar (cumulative totals); no per-round summary needed.
 - Lobby search/filter (MVP): Client-side only; can upgrade to backend query if needed.
 - Root route (`/`): Welcome page with login for non-authenticated; redirect to `/lobby` for authenticated.

## Open Questions
- (All Stage 0 questions resolved — see Decisions section above)

---

## Progress Tracker
Use checkboxes to mark completion. Add brief notes/dates.

- [x] Stage 0 — Align and prepare
  - [x] Wireframes committed (detailed UX spec in Stage 6)
  - [x] Endpoint shortlist complete (pending backend verification)
  - [x] Game phases and data requirements documented
- [x] Stage 1 — App shell and routing (web)
  - [x] Root route `/` redirects authenticated to `/lobby`, shows login for non-authenticated
  - [x] `/lobby` placeholder
  - [x] `/game/[gameId]` placeholder
  - [x] Header updated with Lobby link and Resume CTA placeholder
- [x] Stage 2 — Read-only lobby
  - [x] TypeScript types for game data
  - [x] API client functions with error handling
  - [x] Two separate lists (joinable and in-progress)
  - [x] Loading skeleton, empty states, error banner
  - [x] Refresh button
  - [x] Resume last game CTA (lobby and Header)
  - [x] Client-side search/filter
- [x] Stage 3 — Create and join game
  - [x] Create Game modal with optional name (defaults to "{CreatorName} game")
  - [x] Join action navigating to `/game/[gameId]`
  - [x] Toaster with expandable error details; `traceId` hidden until expanded; log `traceId` in dev
  - [ ] Starting dealer selection (optional; defaults to creator) - deferred to later
- [x] Stage 4 — Read-only game room and table snapshot
  - [x] Snapshot types mirrored in frontend (`GameSnapshot`, phase unions)
  - [x] `/game/[gameId]` renders phase header, seats, trick area, sidebar, hand snapshot
  - [x] Server action fetches `/api/games/{id}/snapshot` with ETag + polling fallback
  - [x] Manual refresh button + subtle polling indicator and error surface in UI
  - [x] Storybook/Vitest coverage for snapshot parsing and view layout states
- [ ] Stage 5 — Core interactions
  - [x] Backend ready/bid/trump/play endpoints and services, including AI orchestration loop and full-game test
  - [x] Backend route/service tests upgraded to shared transaction harness (ready, bid, auth, membership)
  - [x] Frontend ready state wiring (mark ready action, sidebar UX, auto-refresh)
  - [ ] Frontend bid submission UX and server action with pessimistic handling
  - [ ] Legal card gating + play action hookup with optimistic refresh
  - [ ] Host AI seat management controls in game room
  - [ ] Two-tab verification of ready→start→bid→play flow with automated coverage
- [ ] Stage 6 — UX and accessibility
- [ ] Stage 7 — Mobile foundations (Expo)
- [ ] Stage 8 — Mobile interactions

---

## Working Notes
- Environment: ensure `NEXT_PUBLIC_BACKEND_BASE_URL` is set locally; keep `.env` out of git; maintain `.env.example` with placeholders.
- Data sync: Prefer manual refresh initially; add interval polling with conservative cadence (lobby slower, active turn faster).
- Testing: Start with a pair of browser tabs; later, add a small E2E for create→join→bid→play→score.

---

- 2025-11-09: Stage 5 backend foundation — Implemented backend ready/bid/trump/play orchestration (auto-start guard, AI templates, shared transaction test harness) with comprehensive route/service coverage and full-game AI regression test passing.
- 2025-11-09: Stage 5 frontend ready state — Added ready server action, sidebar UX, and pessimistic refresh handling for auto-start flow.
- 2025-11-07: Stage 4 delivered — Read-only game room stitched end-to-end: new `GameRoomView` and client polling shell render snapshots from `/api/games/{id}/snapshot` with ETag awareness, manual refresh, and error surfacing; seat summaries, trick area, sidebar, and hand preview match UX spec; snapshot types mirrored in frontend with Vitest fixtures.
- 2025-11-06: NextAuth security and reliability improvements: Implemented proactive backend JWT refresh (refreshes when missing or within 5 minutes of expiry, including on 'update' trigger). Removed backendJwt from session object (server-only, stored only in JWT token). Added server-only helpers `resolveBackendJwt()` / `requireBackendJwt()` to safely access backend JWT from server components/actions and auto sign the user out when the token is missing or invalid. Env var hardening: `BACKEND_BASE_URL` validation with clear error messages, only throws when refresh is actually needed. Split `BackendApiError` into `lib/errors.ts` (client-safe) and marked `lib/api.ts` as server-only. Added type guards for JWT expiration and backend response validation. All server-only imports properly isolated; no client code can access backend JWT.
- 2025-01-XX: Stage 3 complete — create and join game implemented: Create Game modal with optional name (backend applies default), join action with navigation, toaster with expandable error details and traceId logging. Backend: JWT authentication refactored (JwtExtract middleware, claims/JWT moved under auth module, current_user_db renamed to current_user). Backend: create_game endpoint uses ValidatedJson for request validation. Frontend: cleaned up dead code (api helper, getMe, dashboard), removed duplicate auth logic (centralized in fetchWithAuth), removed client-side default name logic. AUTH_BYPASS support added for debugging (marked for removal). Ready for Stage 4 (read-only game room).
- 2025-01-XX: Stage 2 complete — read-only lobby implemented: TypeScript types, API client functions, game lists with loading/empty/error states, refresh button, Resume CTA in lobby and Header, client-side search/filter. Note: Backend endpoints not yet implemented, so API calls gracefully handle 404s. Ready for Stage 3 (create and join).
- 2025-01-XX: Stage 1 complete — app shell and routing implemented: root route with auth redirect, `/lobby` and `/game/[gameId]` placeholder routes, Header updated with Lobby link and Resume CTA placeholder. Ready for Stage 2 (read-only lobby).
- 2025-01-XX: Stage 0 complete — all MVP decisions documented, endpoints listed, wireframes integrated (detailed UX spec in Stage 6). Ready for Stage 1 implementation.
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

### Scratchpad — working area (temporary)
Use this area to co-develop answers for upcoming steps. When an item is finalized, promote it to the appropriate Stage above and clear it here.

Template
- Context/Question:
- Proposed answer (bullet points):
- Open questions/risks:
- Owner/next action:

Stage 0 complete — all decisions moved to Decisions section and relevant Stages above.

Remaining items for future stages:
- Backend: Verify endpoint names/payloads match decisions above.
- AI behavior: Confirm minimal acceptable AI for testing (random legal vs heuristic).
- Validation: Name length/charset limits for game creation.

