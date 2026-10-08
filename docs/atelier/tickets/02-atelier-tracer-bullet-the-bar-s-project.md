# 02: Atelier tracer bullet: the bar's project name comes from atelier

**What to build:** The thinnest path through every layer: an `atelier` crate in the dotfiles repo, built and installed by the install script, tested in CI on Linux and macOS against a real private tmux server, and wired into the tmux config. The left block shows the resolved project name (grove option, else git root name, else session name) coming from `atelier bar left`. Everything else on the bar is unchanged.

**Blocked by:** None (can start immediately)

**Status:** done

- [x] `atelier --version` works after running the install script on a machine with no Rust toolchain
- [x] A test helper starts a tmux server on a random socket with an empty config and tears it down
- [x] The resolver is tested through that harness for a grove-tagged session, a session in a git repo, a session outside any repo, and a path containing a space
- [x] CI runs fmt, clippy with warnings denied and tests on Linux and macOS, plus shellcheck on the remaining bash
- [x] The tmux status bar shows the project name produced by atelier

## Comments

- `atelier bar left -t <session>` prints only the resolved project name for now.
  `scripts/tmux-status-left` still draws the capsules, padding and the root/wt
  chip (from git); ticket 04 turns `bar left` into the full block. Its tests in
  `atelier/tests/bar_left.rs` assert the bare name and will need updating then.
- The resolver reads `@grove_project` only. `@grove_root` and `@grove_worktree`
  are left for ticket 04, the first consumer of root/worktree.
- tmux.conf now passes `#{q:session_id}` as a 4th argument; the script calls
  atelier only when it is present, so a stale tmux.conf shows the session name
  rather than another session's project.
- `dot_zshrc` puts `~/.cargo/bin` on `PATH`, since rustup is installed with
  `--no-modify-path`.
- Shellcheck runs on `install.sh` and every executable in `scripts/` with a
  sh/bash shebang (`md-preview` is bun and skipped). The two SC2015 findings in
  the status scripts are fixed; nothing is disabled.
- **Unverified:** the rustup download itself (its host is blocked from the
  container this was built in); the `cargo install` half was run against an
  empty `$HOME`, twice, and `~/.local/bin/atelier --version` works. The CI
  workflow has not run (branch not pushed); its commands pass locally on Linux
  with tmux 3.4 and shellcheck 0.9. Nothing was run on macOS. The bar was
  checked end to end by attaching a client inside a nested tmux and capturing
  the rendered status line: git repo, grove-tagged and plain sessions, with and
  without the binary installed.
