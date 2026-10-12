# 30: Daemon review fixes

**What to build:** Findings of the independent reviews that live in
`src/daemon/`, which ticket 29 leaves alone.

**Blocked by:** none

**Status:** done

- [x] The daemon's control client is spawned from `tmux::Tmux`, so the daemon finds `tmux` when started with a `PATH` that lacks it
- [ ] FNV-1a 64 has one implementation shared by the daemon and the task catalog
- [x] The daemon's control-mode attach does not change the session picker's order or its "attached" label
- [x] A watch the repo watcher cannot add (e.g. inotify's `max_user_watches`) is logged, not silent

## Comments

- **tmux lookup.** The control client is now
  `tokio::process::Command::from(Tmux::command())`, so it gets `Tmux`'s
  `PATH`-then-fallback lookup and its `-S`. Tested by
  `the_daemon_runs_when_path_lacks_tmux` (empty `PATH`).
- **FNV not shared yet.** Ticket 29 adds `src/fnv.rs` (`crate::fnv::fnv1a`) and
  switches `tasks/catalog.rs` to it, but has not landed on the base branch, so
  `daemon/mod.rs` keeps its private `fnv1a`. Once 29 is merged: delete it and
  call `crate::fnv::fnv1a` — same constants, so lock/socket/log names do not
  change.
- **Attach and the picker.** tmux 3.4 has no session-less control client and no
  way to attach without counting: the daemon's client is in
  `#{session_attached}` and its attach sets `#{session_last_attached}`. The
  "attached" label was already right — `sessions rows` derives it from
  `list-clients` with `#{client_control_mode}` = 0. Only the order was skewed:
  `attach-session` with no target picks the most recently *active* session,
  which then jumped to the top. The daemon now passes `-t` the session the
  picker already sorts first (greatest `#{session_last_attached}`, then smallest
  name — the picker's own key), so the bump cannot reorder anything, including
  when no session was ever attached. This duplicates the picker's sort key in
  `daemon/server.rs`; if the picker's order changes, `first_in_picker_order`
  must follow. Tested with and without a prior attach. Nothing is left for the
  picker to filter.
- **Watch failures.** A failed `watch` is logged once per path to the daemon's
  stderr (its log file) and retried at every settle; a notify error event is
  logged too. Tested through a tracked directory deleted by hand, which takes
  the same `watch` error path as an exhausted `max_user_watches` (not
  reproduced).
