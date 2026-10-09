# 20: Services on launchd and systemd

**What to build:** One description file declares the background services. `atelier service install` writes launchd agents on macOS or systemd user units and timers on Linux. cc-tap's agents and the preview server move to it; hand-written plists are deleted.

**Blocked by:** 15 (Markdown notes and cursor sync), 17 (Setup: links and stubs)

**Status:** done

- [x] Keep-alive services and the weekday 08:00 update are expressible
- [x] A service is reloaded only when its generated file changed, and started if it is not running
- [x] An unwritable LaunchAgents folder is reported with the command that fixes it
- [x] On Linux, linger is enabled so services survive SSH logout
- [x] Generated files snapshot-tested for both systems

## Comments

- The description file is `services.toml` at the repo root. Each `[[service]]`
  has `label`, `description`, `run` (program relative to `$HOME`, then
  arguments), an optional `log` and an optional `schedule` (`days`, `at`,
  `every` seconds); no schedule means keep-alive. `atelier service
  install|list|uninstall`, with `--system launchd|systemd` to override the OS
  guess and `--repo` like `setup`.
- The generated plists are byte-identical to the deleted hand-written ones: the
  launchd snapshots were seeded from those files before any code existed.
  The systemd snapshots were written by hand first and pass
  `systemd-analyze verify` (bar the missing binaries under `/root`).
- Deviation: systemd units call `%h/<program>` directly rather than through the
  `/bin/sh -c 'exec "$HOME/…"'` wrapper, since `%h` is systemd's own home
  specifier; a service's `log` is macOS-only, systemd output goes to the
  journal. The update timer also carries `OnActiveSec=0` (`RunAtLoad`) and
  `OnUnitActiveSec=1800s` (`StartInterval`), next to the asked-for
  `OnCalendar=Mon..Fri 08:00` and `Persistent=true`.
- Deviation: a changed unit gets `enable` + `restart` rather than `enable
  --now` (which would not apply a new definition to a running service);
  unchanged units get `enable --now`.
- Beyond the ticket: `scripts/cc-tap-service` logs under
  `${XDG_STATE_HOME:-~/.local/state}/cc-tap` and restarts through `systemctl
  --user` off macOS; `atelier preview` on Linux starts the
  `md-preview` unit with `systemctl --user start` (default port only) before
  falling back to a detached server.
- Unverified: nothing ran against a real launchd or systemd user manager —
  `launchctl`, `systemctl` and `loginctl` are fakes on `PATH` in the tests. The
  unwritable-LaunchAgents test skips itself as root (this container), so it
  only runs in CI. Under systemd the preview server's self-restart on a new
  binary is an exit followed by `RestartSec=10`, so it is down for ~10 s.
