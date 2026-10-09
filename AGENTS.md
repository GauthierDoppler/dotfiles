# AGENTS.md

This file provides guidance to AI coding agents (Claude Code, Codex) when working
with code in this repository. `CLAUDE.md` is a symlink to this file. It records
rules and the rationale that still decides things; longer reasoning lives in
[`docs/atelier/design-notes.md`](docs/atelier/design-notes.md). A rule followed
by a test name (`file.rs::test`, under `atelier/`) is enforced by that test.

## Comments

A comment is justified only when it records a constraint imposed by **external
software** that cost real debugging time and would otherwise be re-hit — "fzf's
`unbind` removes a key's behaviour rather than restoring its default", "tmux
allows one popup per client and a nested `display-popup` silently no-ops while
still exiting 0", "tmux truncates the *expanded* format string and will cut a
`#[fg=]` in half", "BSD find has no `-executable`".

Not justified: restating what the line below does, explaining why a particular
option or colour was chosen, narrating a design decision, or justifying a
removal ("there used to be X, dropped because never used"). Rationale of that
kind belongs in this file, not in the config. When changing something, do not
add a comment explaining the change.

## What This Is

Dotfiles for macOS and Linux: zsh, tmux, git, neovim, ghostty, zed, lazygit,
lazydocker and delta, plus the background services (cc-tap, the markdown
preview) declared in `services.toml`. `atelier/` is a Rust binary that owns the
behaviour (status bar, pickers, task runner, preview server, setup, services,
doctor); configs stay plain files linked from the repo.

## Installation & Symlinks

```bash
xcode-select --install
git clone https://github.com/GauthierDoppler/dotfiles.git ~/dotfiles
cd ~/dotfiles && ./install.sh    # first run: SSH key setup, second run: full install
```

`install.sh` is only the bootstrap (Homebrew, building atelier, the keyboard
layout, Node, then `atelier setup`, `claude-settings-sync` and `service
install`); it stops if the atelier build fails.

`atelier setup [--profile desktop|remote] [--dry-run] [--repo PATH]` links from
the `LINKS` and `STUBS` tables in `atelier/src/setup.rs`, the one list of what
goes where; use the `/add-config` skill to add one. It backs up as `*.bak`,
`*.bak.1`, … and is idempotent (`tests/setup.rs`); it refuses a `--repo` that is
not this repo (`setup.rs::a_repo_that_is_not_the_dotfiles_is_refused`) and
prunes only dangling links into the repo
(`setup.rs::only_dangling_links_into_the_repo_in_managed_folders_are_removed`).
Profile `remote` (default off macOS) skips GUI app configs, casks and the
keyboard layout.

**Naming convention:**
- `~/.config/*` folders → stored as-is (e.g., `ghostty/`, `lazygit/`)
- `~/` dotfiles → prefixed with `dot_` (e.g., `dot_zshrc` → `~/.zshrc`)
- `~/.config/*` individual files → keep parent structure (e.g., `git/ignore`)

## Machine-local vs shared config

Every tracked config must be portable — no `/Users/<name>` paths, no per-machine
sockets, no work-specific tooling.

**`~/.zshrc`, `~/.zprofile` and `~/.gitconfig` are stubs, not symlinks**: a
small real file loads the tracked config (`source "$HOME/dotfiles/dot_zshrc"`,
`[include] path = ~/dotfiles/dot_gitconfig`) and machine-local content
accumulates below it. Installers and `git config --global` append to these files;
through a symlink, the appends land in tracked files. For git, later values win.
A stub is written only when its load line is missing
(`setup.rs::local_content_below_an_existing_stub_is_left_alone`).
`dot_tmux.conf` is a plain symlink: nothing appends to it.

| Shared (tracked)              | Local (untracked)                     |
| ----------------------------- | ------------------------------------- |
| `dot_zshrc`                   | `~/.zshrc`, below the load line        |
| `dot_zprofile`                | `~/.zprofile`, below the load line     |
| `dot_gitconfig`               | `~/.gitconfig`, below the include      |
| `dot_claude/settings.json`    | `dot_claude/settings.local.json`      |

`dot_zprofile` holds login-shell **environment** (locale, `JAVA_HOME`, Android
SDK), every block guarded on its tool; `dot_zshrc` holds interactive setup.
`JAVA_HOME` follows the newest JDK (`java_home` with no `-v`); a project pins its
own in its Gradle toolchain.

`atelier local-diff` lists what has accumulated locally; each finding is a
decision: promote it, or leave it local. It walks setup's `STUBS` table, so a new
stub needs no second list (`tests/local_diff.rs`).

**Claude Code settings are a special case.** Claude Code has no user-scope
`settings.local.json` and *rewrites* `~/.claude/settings.json` in place
(reordering keys, absolutising paths), so that file is **not symlinked**:
`atelier claude-settings-sync` generates it as a 3-way merge.

```
drift     = live - snapshot      what the app changed since we last wrote
new local = local * drift        captured, so nothing machine-local is lost
live      = base * new local     base propagates where the app was silent
snapshot  = live                 recorded for next run
```

The snapshot, `dot_claude/settings.generated.json`, is what tells "the app
changed this" from "the base changed under me"; without it a base edit is
captured as local drift and can never propagate again
(`claude_settings.rs::a_base_change_propagates_where_the_app_was_silent`). Both
the override and the snapshot are gitignored. Preview with `--dry-run`.

## Atelier

`atelier/` is a Rust binary crate (edition 2021, clap derive); the plan is
`docs/atelier/spec.md` and `docs/atelier/tickets/`.

- **A feature is a module plus one line.** It lives in `src/<feature>.rs` (or
  `src/<feature>/`), exposes `pub enum Command` deriving `clap::Subcommand` with
  `pub fn run(self, tmux: &Tmux) -> Result<()>`, and is registered by one line in
  the `features!` list in `main.rs`. Top-level commands (`atelier daemon`,
  `atelier status`) go after `; top level:`. A feature with flags and no
  subcommands derives `clap::Args` and calls `crate::flags_only!(Command)`. Keep
  `main.rs` and `Cargo.toml` small; every branch touches them.
- **Shared modules** (`tmux`, `session`, `git`, `shell`, `fzf`, `opener`,
  `process`, `dotfiles`, `fnv`): git only through `git`, other programs through
  `process`, `$HOME` and the checkout through `dotfiles`.
- **tmux only through `tmux::Tmux`**, always on an explicit socket (`--socket`,
  else `$TMUX`). The binary is the first `tmux` on `PATH`, else the
  Homebrew/Linuxbrew/system prefixes, because hooks run with a thin `PATH`
  (`daemon.rs::the_daemon_runs_when_path_lacks_tmux`). `Tmux::resolve` is "the
  given target, else the current one"; `Tmux::watched` is the one "is a terminal
  showing this window" check.
- **Control-mode clients never count as looking.** The daemon is one, so every
  "is someone looking" check skips `#{client_control_mode}` = 1
  (`hook_claude.rs::a_control_mode_client_does_not_count_as_watching` and its
  twins in `tasks.rs`, `sessions.rs`, `bar_daemon.rs`).
- **Every picker builds fzf through `fzf::picker`** (shared flags, colours,
  `j`/`k`/`q` + `i` search mode) and calls back through `fzf::atelier`. The
  pickers look alike on purpose; one that needs to differ adds flags after it.
  Needs fzf ≥ 0.45 (`transform`).
- **Session identity is `session::resolve`, for every consumer**: grove's
  `@grove_*` options, else git from `#{session_path}` (main worktree as root),
  else the session name. Session names are never parsed
  (`sessions.rs::a_hand_named_session_that_looks_like_grove_belongs_to_no_project`).
- **tmux 3.4 does not expand formats in `display-popup`'s command**, so every
  popup picker resolves its own session, pane and client.
- **`display-message -p` returns one field per call** (control characters come
  out as octal, newlines as `_`; numeric fields before one free-text field can be
  split with `splitn`). With an unknown `-t` it exits 0 with empty output, so
  emptiness is not-found. `git -C ""` runs in the cwd — never pass an empty path.
- **Tests drive the built binary against a private tmux server**
  (`tests/common/mod.rs`, `TmuxServer::start()`: `tmux -L atelier-test-<pid>-<n>
  -f /dev/null`, one per test). Assert on what tmux or the binary shows, never on
  internals; pure render functions are the only other seam. A test never
  waits on a child without a deadline (`common::BoundedOutput`, read timeouts
  on sockets), and never reaches a real browser or service manager: the
  opener, `osascript`, `launchctl` and `systemctl` are looked up on `PATH` so a
  test can fake them.
- **Tests describe behaviour, never the code they replaced**: no parity table
  against a removed script, no name that says where a fixture came from.
  Fixtures use made-up names (`my-app`, `com.example.web`), never a real project
  or service; the service and doctor tests read `tests/fixtures/services.toml`,
  and one test alone loads the repo's own
  (`service.rs::the_repo_s_own_services_toml_is_valid`).
- **The daemon (`src/daemon/`) is one per tmux socket**, started by `atelier
  daemon --ensure` from `dot_tmux.conf`
  (`daemon.rs::ensuring_the_daemon_again_keeps_a_single_one`); its files live in
  `$XDG_RUNTIME_DIR/atelier/`. Its only link to tmux is one `tmux -C
  attach-session`; it never creates a session
  (`daemon.rs::the_daemon_creates_no_session_of_its_own`). It pushes the bar, and
  repo counts follow a file watcher, never a timer
  (`bar_daemon.rs::no_git_process_starts_while_the_repo_is_untouched`). Its tests
  turn `automatic-rename` off (tmux applies it lazily). Mechanics: design notes.
- **Installed into `~/.local/bin/atelier`**, called by that absolute path; every
  caller must still work when it is absent.
- **`atelier doctor` turns what this file asks you to remember into checks**
  (`src/doctor.rs`): `ok`, `FAIL` with a one-line `fix:`, `skip`, or `look`. **The
  Nerd Font is `look`, never `ok`**: no terminal reports which font draws a glyph.
- **CI** (`.github/workflows/atelier.yml`): fmt, clippy `-D warnings` and tests on
  Linux and macOS, plus shellcheck. Tests run under cargo-nextest
  (`atelier/.config/nextest.toml`, profile `ci`), which names and kills a hung
  test; the job stops at 15 minutes. It ignores only `docs/`, top-level markdown
  and `nvim/` (`ci.rs::ci_runs_on_every_trigger_for_the_files_the_tests_read`).
  fzf-driven tests skip locally without fzf ≥ 0.45 but fail under `CI`. Run
  `cargo fmt`, `cargo clippy --all-targets -- -D warnings` and `cargo test` in
  `atelier/` before pushing.

## Tmux

**There is no session persistence — no tmux-resurrect, no tmux-continuum, no
tpm — and that is deliberate.** `grove` (see the `grove` skill) is the session
factory: a session is derived from a worktree plus `.grove/config.yaml`, so
recreating it reconstructs what the project *is*, where a save-file restores
whatever was open at the last checkpoint. Do not add a persistence plugin.

**`terminal-features`, not `terminal-overrides`, and never with a bare `set -a`**:
appending to an array option grows it on every `Prefix + r`. `set -gu` first
(restoring the default, which carries the stock clipboard and focus entries),
then `set -as` (`doctor.rs::terminal_features_grown_by_reloads_fail`).

**No terminal app is assumed.** Entries are keyed on the client's TERM:
`xterm-ghostty`, `xterm-kitty`, `wezterm` get `RGB:usstyle:sync` (`usstyle`
makes nvim's undercurl a curl); `xterm-256color` gets `RGB` alone, since
Terminal.app reports it too. iTerm2 needs no entry: tmux recognises it from
XTVERSION. `default-terminal` stays `tmux-256color`.

- `detach-on-destroy off`: killing a session's last window switches session
  instead of ejecting the client.
- `Prefix + 1..9` needs `keyboard/FR-AZERTY-num.bundle` (AZERTY, unshifted
  QWERTY-order digits), **copied** by `install.sh` — macOS's input-source daemon
  does not reliably follow a symlink there. Selecting it stays manual (README).
- No resize bindings, on purpose: drag the border; `Prefix + L` is `next-layout`.
- `monitor-activity` **off** (Neovim and Claude Code kept every window lit);
  `monitor-bell` on, since a BEL is rare enough to mean something.

**Every picker always opens, even with nothing to list**, on a placeholder row
where every action is a no-op (fzf has no non-selectable row), so the session
picker's `Tab` stays reachable
(`sessions.rs::a_project_with_no_other_session_says_so_and_tab_still_reaches_the_others`,
`pick.rs::every_action_on_the_placeholder_row_does_nothing`).

## Session picker

`Prefix + Space` / `Prefix + s` run `atelier sessions pick`; every fzf bind calls
back into `atelier sessions <subcommand>` (`tests/sessions.rs`). `Tab` toggles
project/all. The preview tails the session's current window sized from
`$FZF_PREVIEW_LINES`, since fzf clips the bottom, where the prompt or error is;
previews cannot be focused, so `h`/`l` cycle that session's window
(`sessions.rs::l_and_h_cycle_the_other_sessions_window_and_enter_lands_on_it`).

## Per-project tmux tasks

`Prefix + e` opens `atelier tasks pick` over `<project>/.tmux/`, so build and run
loops are driven from any window, not from Neovim. `.tmux/` is ignored globally
(`git/ignore`); the `tmux-tasks` skill covers writing tasks. **A task is any
executable file at depth 1 or 2** — the executable bit is the only filter, so
`.tmux/lib/common.sh` stays out. Subfolders are groups (`Tab`). Headers come from
the first 20 lines:

```bash
#!/usr/bin/env bash
# task: assemble the debug APK and install it     <- picker description
# tmux: window    window | split-down | split-right | popup | detach
```

- Splits (`split-right 40%`, `split-down 15`, default 30%) are named by direction
  because `-v`/`-h` are inverted between tmux and vim.
- `popup` reuses the picker's popup: tmux allows one popup per client and a
  nested `display-popup` silently no-ops while exiting 0
  (`tasks.rs::a_popup_task_picked_in_the_picker_runs_in_the_picker_s_popup`).
- `window`/`detach` reuse a window named after the task (`android/build` →
  `android-build`); `detach` re-runs with `respawn-pane`, since `respawn-window`
  has no `-d` (`tasks.rs::rerunning_a_detached_task_reuses_its_window_without_selecting_it`).
- Tasks run at the project root from `#{session_path}`, **not** the pane's cwd,
  with `TMUX_TASK_ROOT`/`TMUX_TASK_NAME` set
  (`tasks.rs::from_inside_a_pane_three_directories_deep_the_pane_s_session_is_used`),
  under `atelier tasks exec`, most recently run first.

**`@task_status`** (`running`/`ok`/`fail` → `●`/`✓`/`✗`) goes on the window found
from `$TMUX_PANE`, never the active one, and is reset to `running` each run since
it survives a respawn
(`tasks.rs::rerunning_reuses_the_window_and_resets_the_marker_to_running`). Only
`window` and `detach` are marked and **ring** (`\a`); a split or popup shares the
window you are in (`tasks.rs::a_split_task_neither_marks_nor_rings`), and no bell
sounds in a window a terminal is watching
(`tasks.rs::a_finished_task_stays_silent_in_the_window_being_watched`).

## Claude Code window marker

`atelier hook claude` sets `@claude_status` — `waiting` on `Notification`, `done`
on `Stop`, cleared on `UserPromptSubmit` — rendered as a yellow or green `✻` in
the `@task_status` slot. One command for all three hooks in
`dot_claude/settings.json`; the event comes from `hook_event_name` on stdin, the
window from `$TMUX_PANE`
(`hook_claude.rs::the_pane_s_own_window_is_marked_not_the_session_s_current_one`).

**The command always ends in `|| true`.** Claude Code treats exit 2 as blocking
(on `UserPromptSubmit` it discards the prompt), and 2 is clap's exit on an
unknown subcommand, so a stale atelier would eat every prompt.

A watched window is not marked, and `after-select-window` clears the marker:
going to look dismisses it (`hook_claude.rs::selecting_the_window_clears_its_marker`).

**Notifications policy: the marker is the only signal from Claude Code** — no
bell, `preferredNotifChannel: notifications_disabled`. A beep or banner that
cannot be attributed to a window stops being trusted. Tasks still ring. Ticket 26
may bring back an optional, clickable notification for `waiting`; history in the
design notes.

## One-shot command popup

`Prefix + Enter` opens a popup that takes **one** command, rooted on
`#{session_path}`, so `grove` or `g sync` act on the right repo from any window.
It is a real interactive zsh with a private `ZDOTDIR` at `tmux/oneshot/`:
`preexec` marks that a command ran (a bare Enter does not spend the shot),
`precmd` prints `✓` or `✗ exit N` and waits for one key whatever the outcome, so
a successful `git log` stays readable. It is why the task picker has no
free-form mode.

macOS's `/etc/zshrc` sets `HISTFILE=${ZDOTDIR:-$HOME}/.zsh_history`
unconditionally and oh-my-zsh derives `ZSH_COMPDUMP` the same way, so both would
land in this tracked directory: they are pinned to `$HOME` (`ZSH_COMPDUMP` before
the source, `HISTFILE` after), with a `.gitignore` as backstop.

## Project jump

`project <name> <path> [tab]` in `dot_zshrc` defines a function `<name>` that
`cd`s, renames the terminal tab and `grove attach`es the main worktree's branch.
Registrations (`project biogroup ~/Developer/theodo/biogroup "Biogroup"`) carry
absolute paths, so they are **machine-local**, below the load line of
`~/.zshrc`; `local-diff` reports each one every run, accepted, not filtered. A missing
path is skipped silently, a name that shadows a command *loudly*. **The branch is
derived, never hardcoded to `main`** (first `git worktree list --porcelain`
entry, else `origin/HEAD`); everything after the `cd` is guarded on `grove` and a
git repo. **The tab name is set once**: tmux keeps `set-titles off`, and inside
tmux the OSC 2 goes through passthrough (inner ESCs doubled). Rejected
alternatives are in the design notes.

## cc-tap (Claude Code dashboard + inspector proxy)

[cc-tap](https://github.com/theodo-group/cc-tap) runs as three services from
`services.toml`, driven by `scripts/cc-tap-service`:

| service | runs | when |
| ----- | ---- | ---- |
| `com.theodo.cc-tap.dashboard` | dashboard on `127.0.0.1:3000` | login, kept alive |
| `com.theodo.cc-tap.proxy`     | inspector proxy on `127.0.0.1:8089` | login, kept alive |
| `com.theodo.cc-tap.update`    | `npm install cc-tap@latest` | see below |

`cl` in `dot_zshrc` routes through the proxy when `:8089` accepts a connection,
else plain `claude`; `--no-proxy` forces plain and is stripped.

- **A service manager, not Docker**: `proxy/server.js` hardcodes
  `listen(PORT, '127.0.0.1')`, and reads `~/.claude` anyway.
- **The services bypass the `cc-tap` CLI** (it opens a browser tab per start and
  runs the proxy as the dashboard's child). The proxy writes
  `~/.cc-lens/proxy.json` itself, or the dashboard spawns a second one on
  `:8090`. The dashboard's Stop is undone in ten seconds; really stop with
  `launchctl bootout gui/$(id -u)/com.theodo.cc-tap.proxy` or `systemctl --user
  stop com.theodo.cc-tap.proxy`.
- **Node is fnm's `default` alias**, not Homebrew's; cc-tap needs Node ≥ 24.
- **Updates once per weekday from 08:00, restarting only on a new version** —
  a restart drops every in-flight request through the proxy. The service fires
  at load, every 30 min and at 08:00; the script gates on a stamp
  (`~/.local/share/cc-tap/last-update`) written only on success.

## Services

`services.toml` declares every background service once (no `schedule` means
keep-alive); `atelier service install|list|uninstall` turns it into launchd
agents on macOS and systemd user units on Linux; `tests/service.rs` snapshots
every generated file.

- **launchd**: plists **copied, not linked**. launchd expands neither `~` nor
  `$HOME`, so each runs `/bin/sh -c 'exec "$HOME/…"'`.
- **systemd**: user units with `ExecStart=%h/…`; a schedule adds a `.timer`
  (`Persistent=true`, `OnActiveSec=0`), which is what gets enabled.
- **Files are written and services reloaded only on change**, so re-running
  `install.sh` does not bounce the proxy
  (`service.rs::launchd_install_reloads_only_the_agent_whose_plist_changed`).
- **launchd gotchas**: a `bootstrap` while a booted-out agent lingers fails with
  `5: Input/output error`, so wait for it; a fresh bootstrap can sit at `pended
  nondemand spawn = speculative` for minutes, so always `kickstart` (no `-k`); a
  crash-looping agent stays loaded, so "running" means `state = running`
  (`doctor.rs::a_crash_looping_launchd_agent_fails_although_it_stays_loaded`).
  `~/Library/LaunchAgents` can end up root-owned (Pulse Secure did it); `install`
  fails with the `sudo chown` fix
  (`service.rs::an_unwritable_launch_agents_folder_is_reported_with_the_fix`).
- **systemd gotcha**: without linger, user services die at SSH logout
  (`service.rs::systemd_install_enables_linger_only_when_it_is_off`).

## Skills

`dot_claude/skills/` holds the user-scope skills that document this setup's own
tooling, so they are versioned and portable rather than living loose in
`~/.claude/skills`:

| skill | covers |
| ----- | ------ |
| `tmux-tasks` | writing `<project>/.tmux/` task scripts for the `Prefix + e` picker |
| `grove`      | `.grove/config.yaml` and `.grove/setup.sh` for worktree setup |

`.claude/skills/` holds project-scope skills, active only when working inside
this repo and never linked into `~/.claude`. Besides `add-config`, `to-spec`,
`to-tickets`, `implement`, `code-review` and `tdd` are vendored unchanged from
[mattpocock/skills](https://github.com/mattpocock/skills) (MIT, a copy of the
licence sits in each folder). They expect an issue tracker to have been
configured by that repo's setup skill; this section is that configuration:

- **Issue tracker: local markdown under `docs/<feature>/`.** The spec is
  `docs/<feature>/spec.md`; tickets are one file each at
  `docs/<feature>/tickets/NN-<slug>.md`, numbered from `01` in dependency order,
  with `docs/<feature>/tickets/README.md` as the index. Comments append under a
  `## Comments` heading at the bottom of the ticket.
- **Triage labels** are a `**Status:**` line in each ticket: `ready-for-agent`,
  `needs-human`, `in-progress`, `done`, `wontfix`.
- **Blocking edges** are the `**Blocked by:**` line. The frontier is every ticket
  whose blockers are all `done`.

They are symlinked **one by one** by `atelier setup`, never as a directory:
`~/.claude/skills` also holds skills installed by Claude Code itself (several of
them symlinks into `~/.agents/skills`), and linking the parent would hide them.

**Grove and the tmux tooling stay separate.** Grove is a general worktree
manager and must not grow knowledge of `.tmux/`. The one place they touch is
documented in the `tmux-tasks` skill, not the `grove` one: `.tmux/` is untracked,
so a project that uses both copies it across in its own `.grove/setup.sh`. The
dependency points one way only.

## Neovim Config

`nvim/` is a plain directory (formerly a `kickstart.nvim` submodule, folded in
with `git subtree`). Pre-fold history: `git log <merge>^2`. Upstream kickstart is
not tracked; do not merge from it.

**Structure:** `nvim/init.lua` loads `config.options`, `config.keymaps`,
`config.autocmds`, `config.lazy` in that order; plugins via lazy.nvim in
`lua/config/lazy.lua`, custom ones in `lua/custom/plugins/`, kickstart extras in
`lua/kickstart/plugins/`. **LSP keymaps** live in one `LspAttach` autocommand in
the Telescope section of `lazy.lua`; `grn`/`gra` are Neovim 0.11+ defaults.

**Formatting:** Stylua (`nvim/.stylua.toml`), checked by
`.github/workflows/stylua.yml` at the **repo root** (a workflow under
`nvim/.github/` never runs); run `stylua --check nvim/` before pushing.
**Smoke test:** `nvim/scripts/test-config.sh`.

## Kotlin and Swift (KMP + native mobile)

**Kotlin gets no language server, on purpose.** `kotlin-language-server` is
unmaintained and does not model multiplatform source sets or `expect`/`actual`;
JetBrains' `kotlin-lsp` is pre-alpha and JVM-only; both give diagnostics wrong
often enough to be worse than none. Kotlin in Neovim is treesitter, ripgrep and
ktlint; the rest happens in Android Studio and Xcode. **Do not add a Kotlin LSP,
and do not add DAP for Kotlin or Swift.** Swift's `sourcekit-lsp` is the
exception, added to `servers` in `lsp.lua` **after** `ensure_installed` (mason has
no package) and guarded on `executable('sourcekit-lsp')`.

**Formatters are not in the `Brewfile`**: `ktlint` comes from mason (brew's pulls
a second JDK); `swift-format` runs as `xcrun swift-format` (not shimmed into
`/usr/bin`). `detekt` belongs in the project's Gradle build. The snacks explorer
shows ignored files, so it excludes Gradle/Xcode trees explicitly.

## Key Integrations

- **Tmux ↔ Neovim**: `vim-tmux-navigator` for Ctrl+h/j/k/l, also re-bound in
  `copy-mode-vi` (where `C-j` would otherwise yank and exit). `focus-events on`
  for autoread and gitsigns.
- **Git ↔ Delta**: `dot_gitconfig` includes `delta/themes.gitconfig`; lazygit uses
  delta with side-by-side/inline pagers.
- **Terminal ↔ Tmux**: `extended-keys on` and `S-Enter` re-sending `\x1b[13;2u`,
  so Shift+Enter reaches Claude Code.
- **Tmux modes**: two one-shot modal tables, `Prefix → p` (pane) and `Prefix → t`
  (tab).

## Tmux status bar

`src/bar/left.rs` (project + `root`/`wt`) and `src/bar/right.rs` (key table,
counts, battery, date, clock) are pure, snapshot-tested render functions pushed
by the daemon, with `#()` and then the session name as fallbacks
(`bar_daemon.rs::without_atelier_the_bar_still_shows_the_session_name_and_the_clock`).
The branch is deliberately not shown. Layout reasoning is in the design notes.

- **The window list stays centred only while both blocks are the same width**:
  the left pads to `right::width`, everything on the right is fixed-width (key
  slot reserved at rest, `%a %d %b` never `%-d`, counts padded left)
  (`src/bar/right.rs::the_rendered_block_is_as_wide_as_it_says_in_every_combination`).
  Narrowing sheds whole segments — date, battery, counts — never the window list
  (`src/bar/left.rs::below_the_widest_tier_the_padding_gives_way_to_the_list`).
- **Repo state** (`+412 −89 ↑2 ↓1`) uses plumbing `diff-index`, never `git
  diff`, which rewrites a stat-dirty index even under `--no-optional-locks`
  (`bar_right.rs::reading_the_counts_never_rewrites_the_index`). Counts never
  outgrow 15 cells (`src/bar/right.rs::counts_of_a_thousand_and_more_are_abbreviated`).
- **Widths are terminal cells, never `LANG`-dependent**: tmux runs `#()` with the
  server's environment
  (`bar_right.rs::the_block_is_as_wide_whatever_the_locale_and_the_repo_state`).
- **Truncate inside atelier, never via `status-left-length`**: tmux truncates the
  *expanded* string and will cut a `#[fg=...]` in half.
- **Escape a comma in `#[...]` as `#,` inside a `#{?...}`**, or it ends the branch
  and the rest of the style is silently dropped: `#[bg=#a6d189#,fg=#303446]`.
- **A window has one width active or not**: the inactive format replaces the
  capsules with spaces, and the marker slot is +3 on both.
- **Colour: the window list is greyscale** — blue `#8caaee` is focus,
  yellow/green/red are state. Elsewhere one hue per concept, never reused: mauve
  `#ca9ee6` project, peach `#ef9f76` worktree, flamingo `#eebebe` repo state,
  teal `#81c8be` battery, red `#e78284` low battery. The marker is never drawn on
  the blue. Catppuccin **Frappe**; bar `#414559` on terminal `#303446`.
- **The `` `` caps are U+E0B6 / U+E0B4** (Nerd Font, private-use area):
  some editors silently drop them on write — check with `grep -c` after touching
  a file that holds them.

## Copying out of a pane

- **`Prefix + u`** (`atelier pick popup`) — fuzzy-pick a URL or file path:
  `Enter` opens (file → this session's nvim), `Ctrl-y` copies via OSC 52 (works
  over SSH), `Ctrl-o` hands it to the OS opener, `Ctrl-v` previews markdown.
  Paths that do not exist are dropped, since TUIs truncate long paths
  (`pick.rs::a_claude_code_capture_yields_its_urls_then_the_paths_that_exist`);
  no process per line (`pick.rs::a_full_scrollback_is_listed_without_a_process_per_line`).
- **`Prefix + v`** — copy-mode, then drag, for a **region**. tmux's default
  `MouseDrag1Pane` only selects when the app has not grabbed the mouse (Neovim
  always has); copy-mode owns the mouse and respects pane borders.

## Markdown preview

`<leader>mr` runs `atelier preview <file>` (focus or open its browser tab — tab
reuse is JXA, macOS only); the page re-renders on save and follows the nvim
cursor. Its purpose is **annotation**: "copy all" yields `path` + `L12-L14:
comment` lines for an agent. `Prefix + m` picks a markdown file of the session's
checkout (`tests/markdown_picker.rs`).

- **One permanent server**, `atelier preview serve`, run as a service on
  `127.0.0.1:33440`; the URL path *is* the file's absolute path, so the origin
  never changes. `atelier preview` starts it when `/__meta` does not answer.
- **The page is compiled into the binary** (`atelier/assets/preview/`), so
  **editing it means rebuilding atelier**; the server restarts itself when its
  binary changes (`preview.rs::the_server_restarts_from_its_binary_when_it_is_replaced`)
  via `launchctl kickstart -k`, since a `KeepAlive` respawn is deferred for
  minutes (`pended nondemand spawn = inefficient`).
- **Notes live in `~/.local/share/md-preview/notes/<sha1 of path>.json`**, never
  `localStorage` (`preview.rs::a_notes_file_on_disk_is_read_back_unchanged`).
- **Untrusted markdown on an origin that can read local files** is fenced on
  four sides (`tests/preview.rs`): DOMPurify and mermaid `strict`; a
  `script-src 'self'` CSP with `nosniff`; a `Host` check against DNS rebinding;
  non-markdown files only for a markdown `Referer` in the repo the page's URL
  sits in (not where a symlinked page points), never a hidden file or folder.
  Writes require `content-type: application/json` (no preflight is ever
  answered). These fences stop web pages, not local processes: the server has
  no authentication, so keep it off a machine other users can log into.

## Theming

Catppuccin across the stack: **Frappe** for ghostty, tmux and neovim; Mocha for
lazygit. `ghostty/config` pins `font-family = JetBrainsMono Nerd Font Mono` —
without it ghostty falls back to a font with no Nerd Font coverage and every
powerline separator and icon renders blank. `have_nerd_font = true` in neovim
depends on that pin.
