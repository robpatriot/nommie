# Dev Roadmap

This document tracks the current work plan for building the Nommie UI on web (Next.js) and, later, a mobile client. It captures stages and progress.

---

## Work Items

### Stage 1 — UX fit and polish
- Learning goals: motion for feedback, UI refinement
- Deliverables:
  - [ ] Subtle animations for plays and trick wins
  - [ ] UI polish and refinement
  - [ ] Implement Design v1: Apply the first-endorsed product design across Nommie (typography, spacing, components)
  - [ ] Separate Game Config vs Play UI: Split the game experience into a configuration surface (seating, AI seats, options) and an in-game surface focused on play
  - [ ] Last Trick UI: Persist the most recent trick as a compact card row so play can continue immediately after the final card
  - [ ] User Options: Add per-account settings (e.g., theme, gameplay preferences) surfaced via a profile/options view
  - [ ] Card Play Confirmation Toggle: Provide a per-account option for confirming card plays before submission
- DoD: Smooth animations; polished UI experience; core screens match design reference; users transition smoothly between config and play areas; previous trick reviewable; account preferences persist; card confirmation toggle works

### Stage 2 — Mobile client (Expo)
- Learning goals: React Native/Expo basics; shared types and API; mobile layouts/gestures
- Deliverables:
  - [ ] `apps/mobile` (Expo) scaffold
  - [ ] Shared `packages/shared` for types and API wrapper
  - [ ] Read-only lobby list screen
  - [ ] Render simplified game snapshot
  - [ ] Bid/play interactions with pessimistic writes
- DoD: Two devices can join, bid, and play a trick
