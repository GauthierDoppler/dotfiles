# 11: Daemon tracer bullet: atelier knows the live tmux state

**What to build:** tmux starts `atelier daemon --ensure`; the daemon holds one control-mode connection, builds its view of sessions and windows, keeps it current from events, and exits with tmux. `atelier status` prints that view.

**Blocked by:** 02 (Atelier tracer bullet: the bar's project name comes from atelier)

**Status:** ready-for-agent

- [ ] Starting tmux starts exactly one daemon per tmux socket, even when the config is reloaded
- [ ] Creating, renaming and killing sessions and windows is reflected in `atelier status` without polling
- [ ] The daemon exits when the tmux server exits
- [ ] CLI commands still work when the daemon is not running
- [ ] Control-mode parsing covered by recorded transcripts and by the harness
