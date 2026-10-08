# 02: Atelier tracer bullet: the bar's project name comes from atelier

**What to build:** The thinnest path through every layer: an `atelier` crate in the dotfiles repo, built and installed by the install script, tested in CI on Linux and macOS against a real private tmux server, and wired into the tmux config. The left block shows the resolved project name (grove option, else git root name, else session name) coming from `atelier bar left`. Everything else on the bar is unchanged.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] `atelier --version` works after running the install script on a machine with no Rust toolchain
- [ ] A test helper starts a tmux server on a random socket with an empty config and tears it down
- [ ] The resolver is tested through that harness for a grove-tagged session, a session in a git repo, a session outside any repo, and a path containing a space
- [ ] CI runs fmt, clippy with warnings denied and tests on Linux and macOS, plus shellcheck on the remaining bash
- [ ] The tmux status bar shows the project name produced by atelier
