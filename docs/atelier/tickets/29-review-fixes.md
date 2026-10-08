# 29: Review fixes

**What to build:** Findings of the independent standards and spec reviews of the
branch at `43deb85`, except those that need `src/daemon/` changes (ticket 12's
follow-ups own that: the daemon's own `tmux` spawn and the daemon half of the
shared FNV helper).

**Blocked by:** none

**Status:** done

### Standards

- [x] `atelier doctor` reads the tmux version through `tmux::Tmux`, so it finds the same binary every other command does
- [x] AGENTS.md no longer calls the daemon "planned"
- [x] AGENTS.md agrees with the code on desktop notifications (none yet; ticket 26)
- [x] AGENTS.md says the launchd log comes from the `log` key of `services.toml`
- [x] "the given target, else the current one" is one `Tmux` helper treating empty output as not found
- [x] "is someone looking at this window" is one function, used by the Claude hook and the task runner
- [x] `Prefix + u` no longer passes a `#{pane_id}` that `display-popup` does not expand
- [x] FNV-1a 64 lives in one shared helper (the task catalogue uses it; the daemon can switch later)
- [x] uid read one way; the preview starts its server through `service`, its label pinned to `services.toml` by a test
- [x] one subprocess helper for "run and read stdout" / "run and check success" (the tty-bound `stty` stays, see Comments)
- [x] git is called only through `git.rs`
- [x] the macOS preview opens through `opener` (Chrome tab reuse stays)
- [x] `$HOME` and the repo root are resolved in one place, in one order
- [x] the task runner calls back through `fzf::atelier`, keeping `--socket`
- [x] `status-right` falls back like `status-left` when atelier is absent (argument shapes kept, see Comments)
- [x] shared test helpers live in `tests/common`
- [x] nvim's preview error prefix says `atelier preview`

### Spec

- [x] the session picker orders and labels sessions by terminal clients only, so the daemon's control client does not skew it (labels here, ordering by ticket 30 on the daemon side: see Comments)
- [x] launchd's `running` in `atelier doctor` comes from the agent's state, so a crash-looping agent is not `ok`
- [x] repo counts are clipped to their segment width, with a snapshot test
- [x] ticket 04's padding claim matches the code
- [x] the spec says where linger is enabled and what stays in bash
- [x] CI runs on changes to every file the tests read, and shellchecks `dot_claude/statusline-custom.sh`
- [x] the spec's Linux preview line matches the code, and a detached fallback server is not left to block the service
- [x] AGENTS.md records ticket 28's picker look as a deliberate unification

## Comments

Each finding was re-checked against the code at the start of this ticket
(tickets 12 and part of 13 had landed since `43deb85`).

**Fixed.**

- `src/process.rs` holds `output`/`stdout_of`/`succeeds` and `uid()`; doctor and
  service use it, and `id -u` is gone (libc became an unconditional dependency,
  since `--system launchd` also runs on Linux). Doctor reads the tmux version
  through `Tmux::version`, so it finds tmux in the same fallback prefixes
  (`a_tmux_outside_path_is_still_found`).
- launchd `running` is `state = running` from `launchctl print` for keep-alive
  agents; a scheduled one only needs to be loaded. `service list` shows the
  state (`running`, `spawn scheduled`, `not loaded`). The fakes now print a
  `launchctl print` body; the real output's layout (one tab, `state = `) is
  from memory and unverified on a Mac.
- `Tmux::resolve(target, format)` replaces the four "target else current"
  copies and fails on empty output. `pick` now errors on a `-t` that is not a
  pane instead of silently using the current one, so `Prefix + u` no longer
  passes the unexpanded `#{pane_id}`; the test that pinned the old fallback was
  replaced by `without_a_target_the_current_pane_is_listed` and
  `a_target_that_is_not_a_pane_is_an_error`.
- `Tmux::watched(window)` is the hook's check (a terminal client whose current
  window is this one); the task runner's "active window + a client on the
  session" meant the same except for grouped sessions, where the new one is
  the correct one.
- `src/fnv.rs` with the published FNV-1a 64 vectors; the task catalogue and,
  after merging ticket 30, the daemon's lock/socket names use it (same
  constants, so the names do not change).
- The preview starts and restarts its server through `service::{installed,
  start, restart}`. `LABEL` and `DEFAULT_PORT` stay constants in the preview,
  pinned to `services.toml` by a unit test rather than read from it at runtime
  (that would need the dotfiles checkout just to open a preview).
- Spec 7: a server not under launchd now re-executes its own binary in place
  when it changes instead of exiting, so a detached fallback keeps serving and a
  systemd one keeps its pid (`the_server_restarts_from_its_binary_when_it_is_replaced`).
  `atelier preview` no longer spawns a detached server on the default port when
  the service is installed but will not start; it fails naming the service, so
  nothing ends up holding the unit's port. On macOS this also means an
  uninstalled agent now gets a detached server instead of an error. A detached
  server started *before* `atelier service install` still holds 33440 until it
  is killed; not handled.
- git is only run by `git.rs` (`toplevel`, `files`, `config_entries`), `$HOME`
  and the checkout by `src/dotfiles.rs`, in one order for every command:
  `--repo`, `$DOTFILES`, git root, `~/dotfiles` (so setup, services, doctor and
  claude-settings-sync now honour `$DOTFILES` too).
- The task runner's window commands start with `fzf::atelier_argv`, so they
  carry `--socket` like every other callback.
- `status-right` falls back to `%H:%M` (tmux expands strftime inside `#()`
  before running it; `without_atelier_the_bar_still_shows_the_session_name_and_the_clock`).
- Repo counts: `1k`/`98k`/`2M` past 999, then parts dropped from the right
  behind `…` until the segment fits its 15 cells; snapshots `large_counts` and
  `huge_counts`, both also in the every-combination width test.
- CI: `paths-ignore` (`docs/**`, `*.md`, `nvim/**`) instead of a `paths`
  allow-list, and shellcheck covers `dot_claude/statusline-custom.sh`;
  `tests/ci.rs` reads the workflow and fails if a file the tests read is
  ignored or if a `paths:` filter comes back. The workflow itself was not run
  from here.
- Ticket 04's padding checkbox now says what was delivered (the bash's
  `2 × right + 36` rule); the spec says linger is enabled by `atelier service
  install`, that the Linux preview hands its URL to `xdg-open`, and what stays in
  `install.sh`. AGENTS.md: daemon no longer "planned", one story on
  notifications (none until ticket 26), the `log` key, the picker look recorded
  as ticket 28's deliberate unification.

**Declined.**

- Standards 11, the `stty` closure in the task runner stays: it must inherit the
  pane's tty as stdin, which is exactly what the shared helpers do not do.
- Standards 16, `bar left -t <session>` vs `bar right <path>` stays: the right
  block needs only the path, and taking `-t` would add a tmux round trip to a
  fallback that runs every status interval. Only the missing fallback was a bug.
- Spec 1, ordering. The attached label already counts terminal clients only
  (ticket 12; `a_session_seen_only_by_a_control_client_is_not_marked_attached`).
  Reproduced on tmux 3.4: `attach-session` with no target, as the daemon runs
  it, picks the most recently *active* session and bumps its
  `session_last_attached`. The skew is confined to that one session, already the
  most recently used, and lasts until a terminal client attaches elsewhere.
  Ordering by terminal-client activity would not remove it (the daemon's seat
  usually has no terminal client to take an activity from), so no picker-side
  workaround was added; ticket 30 fixed the skew on the daemon side.

**Not done here (daemon, ticket 30):** standards 1
(`tokio::process::Command::from(tmux.command())` in `src/daemon/server.rs`) and
the daemon's uid read from the socket's owner.
