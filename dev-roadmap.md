# Dev Roadmap

This document tracks the current work plan for building the Nommie UI on web (Next.js) and, later, a mobile client. It captures stages and progress.

---

# Next Steps

- Seem to be getting issues with start up - sometimes errors in logs and sometimes fetch of /api/ws-token fails because backend isn't ready yet - there is code in place to deal with this - we need to use it 
- After lots of actions many errros in frontend logs
- Errors in postgres logs at end of startup
- For above errors where are corresponding logs for local development (e.g. postgres or frontend)

- out of date package warning in pnpm lint

- in dev-db and local-prod certs are generated every time into /opt but then only copied if they don't exist in /var/lib/postgre... which means they will almost always differ which is confusing
- Add redis tests
- fix stop process
- Fix initial load of lobby games list
- lag on all page loads
- Test no interaction for extended period then continuing
- Complete mobile design - Milestone 17

