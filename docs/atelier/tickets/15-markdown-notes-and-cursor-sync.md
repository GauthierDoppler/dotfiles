# 15: Markdown notes and cursor sync

**What to build:** Margin notes and nvim cursor following work on the Rust server, existing notes load unchanged, and the Bun server and its dependencies are deleted.

**Blocked by:** 14 (Markdown preview served by atelier)

**Status:** ready-for-agent

- [ ] Notes read from and written to the same directory and file naming as today
- [ ] "Copy all" output unchanged
- [ ] nvim cursor moves scroll the page only when the line leaves the middle of the viewport
- [ ] Bun server, package files and `node_modules` handling removed, `AGENTS.md` updated
