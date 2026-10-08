# 21: atelier doctor

**What to build:** `atelier doctor` checks what `AGENTS.md` currently asks me to remember, and prints the fix for each failure.

**Blocked by:** 17 (Setup: links and stubs), 20 (Services on launchd and systemd)

**Status:** ready-for-agent

- [ ] Terminfo exists for the current `$TERM`, with the command to copy it from another machine
- [ ] Nerd Font glyphs render, tmux is recent enough, extended keys are on
- [ ] Services are running; on Linux, linger is enabled
- [ ] Exit code is non-zero when a check fails
