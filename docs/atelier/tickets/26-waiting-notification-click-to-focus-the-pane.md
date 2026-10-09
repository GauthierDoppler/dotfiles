# 26: Waiting notification, click to focus the pane

**What to build:** When Claude waits and no focused client shows its pane, post one notification naming project and worktree. Clicking it switches tmux to the right session, window and pane and brings the terminal app forward.

**Blocked by:** 25 (Spike: clickable notification from a daemon)

**Status:** ready-for-agent

- [ ] Only for waiting, once per wait, never for done
- [ ] Not posted when a focused client already shows the pane
- [ ] The most recently active non-control-mode client is switched
- [ ] The notification section of `AGENTS.md` rewritten to explain why they are back
