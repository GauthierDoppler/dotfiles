# 10: Token picker

**What to build:** `Prefix + u` picks a URL or file path from the current pane through atelier: open, copy through OSC 52, or hand to the OS opener. The bash picker is deleted.

**Blocked by:** 02 (Atelier tracer bullet: the bar's project name comes from atelier)

**Status:** ready-for-agent

- [ ] Candidates extracted in one pass over the capture
- [ ] Path candidates that do not exist are dropped
- [ ] Opening a file targets the nvim of this session, or a new nvim window at the session root
- [ ] Copy works over SSH
- [ ] Bash script removed, `AGENTS.md` updated
