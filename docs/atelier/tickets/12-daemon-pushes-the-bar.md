# 12: Daemon pushes the bar

**What to build:** The bar stops running scripts. The daemon renders both blocks and sets them as session options on session switch, client resize and marker changes; the tmux config shows them and falls back to the `#()` call when they are unset.

**Blocked by:** 04 (Left block of the bar rendered by atelier), 11 (Daemon tracer bullet: atelier knows the live tmux state)

**Status:** ready-for-agent

- [ ] Switching session updates both blocks without any `#()` call
- [ ] Resizing the client re-renders at the new width tier
- [ ] Killing the daemon leaves a working bar through the fallback
- [ ] The control-mode client is never counted as someone looking at a window
