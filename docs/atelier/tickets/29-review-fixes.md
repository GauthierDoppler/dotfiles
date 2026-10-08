# 29: Review fixes

**What to build:** Findings of the independent standards and spec reviews of the
branch at `43deb85`, except those that need `src/daemon/` changes (ticket 12's
follow-ups own that: the daemon's own `tmux` spawn and the daemon half of the
shared FNV helper).

**Blocked by:** none

**Status:** in-progress

### Standards

- [ ] `atelier doctor` reads the tmux version through `tmux::Tmux`, so it finds the same binary every other command does
- [ ] AGENTS.md no longer calls the daemon "planned"
- [ ] AGENTS.md agrees with the code on desktop notifications (none yet; ticket 26)
- [ ] AGENTS.md says the launchd log comes from the `log` key of `services.toml`
- [ ] "the given target, else the current one" is one `Tmux` helper treating empty output as not found
- [ ] "is someone looking at this window" is one function, used by the Claude hook and the task runner
- [ ] `Prefix + u` no longer passes a `#{pane_id}` that `display-popup` does not expand
- [ ] FNV-1a 64 lives in one shared helper (the task catalogue uses it; the daemon can switch later)
- [ ] uid read one way; the preview asks `service` for its label and port instead of duplicating them
- [ ] one subprocess helper for "run and read stdout" / "run and check success"
- [ ] git is called only through `git.rs`
- [ ] the macOS preview opens through `opener` (Chrome tab reuse stays)
- [ ] `$HOME` and the repo root are resolved in one place, in one order
- [ ] the task runner calls back through `fzf::atelier`, keeping `--socket`
- [ ] `status-right` falls back like `status-left` when atelier is absent
- [ ] shared test helpers live in `tests/common`
- [ ] nvim's preview error prefix says `atelier preview`

### Spec

- [ ] the session picker orders and labels sessions by terminal clients only, so the daemon's control client does not skew it
- [ ] launchd's `running` in `atelier doctor` comes from a PID, so a crash-looping agent is not `ok`
- [ ] repo counts are clipped to their segment width, with a snapshot test
- [ ] ticket 04's padding claim matches the code
- [ ] the spec says where linger is enabled and what stays in bash
- [ ] CI runs on changes to every file the tests read, and shellchecks `dot_claude/statusline-custom.sh`
- [ ] the spec's Linux preview line matches the code, and a detached fallback server is not left to block the service
- [ ] AGENTS.md records ticket 28's picker look as a deliberate unification
