# 14: Markdown preview served by atelier

**What to build:** `atelier preview serve` replaces the Bun server with the same page, libraries and routes, and `atelier preview <file>` opens or focuses the file's browser tab. The existing service and the nvim mapping point at the new commands.

**Blocked by:** 02 (Atelier tracer bullet: the bar's project name comes from atelier)

**Status:** done

- [x] Page, vendored libraries and mermaid embedded in the binary; works offline
- [x] Re-render on save through server-sent events
- [x] Every security fence ported and tested over HTTP: sanitised HTML, CSP, Host check, Referer rule for non-markdown files, JSON-only writes
- [x] Tab reuse on macOS; the URL is printed on other systems
- [x] Server restarts itself when its binary changes

## Comments

- **Notes and cursor routes ported here, not in 15.** They are a few lines each
  (`GET`/`PUT /__notes/`, `POST /__cursor/` plus the `cursor` SSE event), and
  pointing the service and nvim at atelier without them would have broken notes
  between the two tickets. They are tested over HTTP, including that a notes file
  written by the Bun server reads back unchanged and that a written file lands at
  `~/.local/share/md-preview/notes/<sha1 of path>.json` in the same format. What
  15 has left: delete `scripts/md-preview/` (moving `index.html` and `app.js`
  next to `atelier/assets/preview/lib/` and fixing the two `include_bytes!` paths
  in `src/preview/server.rs`), drop the `md-preview` link, the bun install phase
  and the `.gitignore` line from `install.sh`/the repo, and verify the cursor
  scroll behaviour in a browser.
- **The Bun server is untouched but unused**: the launchd plist runs
  `~/.local/bin/atelier preview serve` and `<leader>mr` calls
  `~/.local/bin/atelier preview <file>`. `index.html` and `app.js` stay in
  `scripts/md-preview/` as the single copy, embedded from there.
- **Libraries vendored** as the exact files the Bun server served from
  `node_modules`, at the `bun.lock` versions, into `atelier/assets/preview/lib/`
  (≈2.9 MB, mermaid is 2.5 MB of it), with their licence files
  (markdown-it-anchor ships none; it is Unlicense).
- **Deviation: CSP and `nosniff` on every response**, not only the page. An
  `.html` or `.svg` in the repo, served to a page, could otherwise run inline
  script on the preview origin. Tested.
- **Deviation: on Linux `atelier preview <file>` starts a detached server** when
  none answers, since there is no service manager integration until ticket 20.
  On macOS it kickstarts the launchd agent as before.
- **Self-restart** compares the mtime and size of the binary's path at startup;
  under launchd (`XPC_SERVICE_NAME` is the label) it runs `launchctl kickstart
  -k`, elsewhere it exits 0. The exit path is tested by replacing a copied
  binary.
- **Unverified (no macOS, no browser here):** the JXA tab reuse, the launchd
  kickstart paths, and the page actually rendering in Chrome from the embedded
  files. The HTTP tests check every script the page references is served with a
  JavaScript type, but nothing executed them.
- **Code review** (both axes run by hand, no sub-agent tool here): one
  Standards finding fixed (file change detection and binary change detection
  duplicated a metadata helper; both now compare mtime and size). Spec axis: no
  missing requirement; the two deviations above are the only additions.
