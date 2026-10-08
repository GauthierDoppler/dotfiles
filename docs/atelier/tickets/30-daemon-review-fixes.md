# 30: Daemon review fixes

**What to build:** Findings of the independent reviews that live in
`src/daemon/`, which ticket 29 leaves alone.

**Blocked by:** none

**Status:** in-progress

- [x] The daemon's control client is spawned from `tmux::Tmux`, so the daemon finds `tmux` when started with a `PATH` that lacks it
- [ ] FNV-1a 64 has one implementation shared by the daemon and the task catalog
- [x] The daemon's control-mode attach does not change the session picker's order or its "attached" label
- [ ] A watch the repo watcher cannot add (e.g. inotify's `max_user_watches`) is logged, not silent
