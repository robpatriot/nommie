# Dev Roadmap

This document tracks the current work plan for building the Nommie UI on web (Next.js) and, later, a mobile client. It captures stages and progress.

---

# Next Steps

- Complete clean up of issues now main is a binary and not a library - Note: this will mean migration-cli cannot compile till separation is completed
    - Run all tests including full game
    - Test with UI. Issues found so far:
        - UI doesn't update on adding AI
        - Marking I'm ready and then reloading page shows not ready (may be true on main as well)
- Separate migration code from backend and put in shared library which migrate-cli can depend on
- Review all dead_code items and determine solution
- Complete mobile design - Milestone 17

