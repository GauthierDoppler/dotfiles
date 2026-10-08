# 15: Markdown notes and cursor sync

**What to build:** Margin notes and nvim cursor following work on the Rust server, existing notes load unchanged, and the Bun server and its dependencies are deleted.

**Blocked by:** 14 (Markdown preview served by atelier)

**Status:** done

- [x] Notes read from and written to the same directory and file naming as today
- [x] "Copy all" output unchanged
- [x] nvim cursor moves scroll the page only when the line leaves the middle of the viewport
- [x] Bun server, package files and `node_modules` handling removed, `AGENTS.md` updated

## Comments

- The notes and cursor routes were already ported in ticket 14. This ticket
  verified them against the Bun server before deleting it: both servers were run
  side by side and sent the same `PUT` the page sends; the notes file (name and
  bytes) and the `GET` response were identical. That Bun-written file is now
  `atelier/tests/fixtures/preview/notes-written-by-bun.json`, and
  `tests/preview.rs` checks the Rust server reads it back and writes it byte for
  byte.
- Known divergence, not reachable from the page: numbers the page would never
  send (`2.0`, `-0`, integers above 2^53, `0.000001`) are re-spelled by
  serde_json (`2.0` kept, `1e-6`, …) where Bun printed them JS-style. Notes only
  carry integer line numbers, strings and booleans, all serialised by
  `JSON.stringify` before they reach the server.
- "Copy all" and the cursor's middle-of-viewport rule run in the browser, which
  the HTTP seam cannot drive. They are pinned by asserting the served `app.js`
  still contains the relevant lines, and the copy-all logic was run once under
  node on the served notes (`path`, blank line, `L3: …`, `L12-L14 (orphaned): …`).
  The page itself was moved with `git mv`, unchanged.
- `index.html` and `app.js` moved to `atelier/assets/preview/`; editing them now
  needs a rebuild, documented in `AGENTS.md`.
- Bun itself is still installed by `install.sh`: grove is a bun project. Only
  the `bun install` for `scripts/md-preview` went.
- A machine set up before this change keeps a dangling `~/.local/bin/md-preview`
  symlink; `atelier setup` does not prune links it no longer manages. Remove it
  by hand.
- Unverified on macOS: nothing in this ticket is platform-specific, but the
  launchd path was not exercised here.
