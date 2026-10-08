# 14: Markdown preview served by atelier

**What to build:** `atelier preview serve` replaces the Bun server with the same page, libraries and routes, and `atelier preview <file>` opens or focuses the file's browser tab. The existing service and the nvim mapping point at the new commands.

**Blocked by:** 02 (Atelier tracer bullet: the bar's project name comes from atelier)

**Status:** ready-for-agent

- [ ] Page, vendored libraries and mermaid embedded in the binary; works offline
- [ ] Re-render on save through server-sent events
- [ ] Every security fence ported and tested over HTTP: sanitised HTML, CSP, Host check, Referer rule for non-markdown files, JSON-only writes
- [ ] Tab reuse on macOS; the URL is printed on other systems
- [ ] Server restarts itself when its binary changes
