# Dev Roadmap

This document tracks the current work plan for building the Nommie UI on web (Next.js) and, later, a mobile client. It captures stages and progress.

---

# Next Steps

- Regenerate Cargo.lock from Cargo.toml and add to repo
- Confirm dev-db/local and local-prod setup work correctly including:
    - SSL process (all 5 scenarios - can store on a branch to share or move to scripts) 
    - Changes to docker build process - contexts/dockerignore/file mounting
- If the database is rebuilt so the user that is currently logged in is not in the db then we should log out
- fix stop process
- Fix initial load of lobby games list
- lag on all page loads
- Test no interaction for extended period then continuing
- Complete mobile design - Milestone 17

