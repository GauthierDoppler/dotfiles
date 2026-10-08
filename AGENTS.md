# AGENTS.md

This file provides guidance to AI coding agents (Claude Code, Codex) when working
with code in this repository. `CLAUDE.md` is a symlink to this file.

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

A macOS dotfiles repo managing configs for: zsh, tmux, git, neovim, ghostty, zed, lazygit, lazydocker, and delta, plus the background services (cc-tap, the markdown preview) declared in `services.toml`. All configs are symlinked from this repo to their expected locations by `atelier setup`.

## Installation & Symlinks

```bash
xcode-select --install
git clone https://github.com/GauthierDoppler/dotfiles.git ~/dotfiles
cd ~/dotfiles && ./install.sh    # first run: SSH key setup, second run: full install
```

The script is phased: SSH key → Xcode CLT → Homebrew → brew bundle (Brewfile) → Oh My Zsh → atelier → `atelier setup` → keyboard layout → Claude settings merge → bun → Node LTS → npm globals → `atelier service install` → app registration.

`install.sh` is only the bootstrap. Links and stubs are `atelier setup
[--profile desktop|remote] [--dry-run] [--repo PATH]`, whose `LINKS` and `STUBS`
tables in `atelier/src/setup.rs` are the one list of what goes where. It backs up
an existing file or folder as `*.bak` (then `*.bak.1`, … — an earlier backup is
never overwritten), replaces a symlink pointing elsewhere without one, and prints
only what it changes, so a second run says `setup: up to date`. The repo is
`--repo`, else the current git root, else `~/dotfiles`, and must look like this
repo (`dot_zshrc` plus `atelier/Cargo.toml`) so running it from another project
cannot link that project into `$HOME`. It also removes ("pruned:") symlinks left
by an older install, and only those: a link directly inside a folder it links
into, whose target resolves inside the repo and no longer exists — what deleting
a script leaves behind in `~/.local/bin`. `tests/setup.rs` runs it against a
temporary `$HOME`, twice. When adding a new config, use the `/add-config` skill.

**Profiles.** `desktop` (the default on macOS) is everything; `remote` (the
default elsewhere, and `./install.sh --profile remote`) skips GUI app configs
(`desktop(...)` entries in `LINKS`: ghostty, zed), Brewfile casks and the keyboard
layout. Launch agents and app registration are gated on macOS, not on the
profile.

**Naming convention:**
- `~/.config/*` folders → stored as-is (e.g., `ghostty/`, `lazygit/`)
- `~/` dotfiles → prefixed with `dot_` (e.g., `dot_zshrc` → `~/.zshrc`)
- `~/.config/*` individual files → keep parent structure (e.g., `git/ignore`)

## Machine-local vs shared config

Every tracked config must be portable — no `/Users/<name>` paths, no per-machine
sockets, no work-specific tooling.

**`~/.zshrc`, `~/.zprofile` and `~/.gitconfig` are stubs, not symlinks.**
`atelier setup` writes a small real file that loads the tracked config, and
everything machine-local accumulates below that line:

```sh
# ~/.zshrc
source "$HOME/dotfiles/dot_zshrc"
```
```sh
# ~/.zprofile
source "$HOME/dotfiles/dot_zprofile"
```
```gitconfig
# ~/.gitconfig
[include]
	path = ~/dotfiles/dot_gitconfig
```

This exists because third-party installers append to `~/.zshrc` and
`~/.gitconfig` directly — through a symlink, those appends land in tracked
files, which is how a `/Users/<name>` socket path once ended up staged here.
`git config --global` writes to the stub too, which is now correct. For git,
later values win, so anything below the include overrides the shared config.

A stub is idempotent by design: if the last line of its load block (the one
carrying the path — a bare `[include]` would match any include) is already
present it writes nothing, so re-running setup never touches accumulated local
content. The path is written through `$HOME` (`~` for git) when the repo is under
it, so the stub itself carries no machine-specific path. There are no paired
`*.local` files any more — setup folds legacy `~/.zshrc.local` and
`~/.gitconfig.local` into the stub on the next run and keeps them as
`*.local.migrated`.

| Shared (tracked)              | Local (untracked)                     |
| ----------------------------- | ------------------------------------- |
| `dot_zshrc`                   | `~/.zshrc`, below the load line        |
| `dot_zprofile`                | `~/.zprofile`, below the load line     |
| `dot_gitconfig`               | `~/.gitconfig`, below the include      |
| `dot_claude/settings.json`    | `dot_claude/settings.local.json`      |

`dot_zprofile` holds login-shell **environment** — locale, `JAVA_HOME`, the
Android SDK — while `dot_zshrc` holds interactive shell setup. Both files are
loaded by a tmux pane, so the split is a convention, not a functional boundary.
It exists because that env was living in an untracked `~/.zprofile` that
`local-diff` did not watch: a config a new machine silently lacks, with nothing
to surface the gap. Every block is guarded on the tool being present, so the file
is inert on a machine with no JDK and no Android SDK.

`JAVA_HOME` comes from `/usr/libexec/java_home` with no `-v`, so it follows the
newest installed JDK rather than hardcoding a path this repo cannot make
portable. A project needing a specific JDK pins it in its own Gradle toolchain
config, or in `~/.zprofile` below the load line.

`dot_tmux.conf` is still a plain symlink: nothing appends to `~/.tmux.conf`.

Run `atelier local-diff` (`local-diff` is a `dot_zshrc` alias for it) to list
what has accumulated locally. Each finding is a decision: promote it into the
tracked config, or leave it local. It walks the same `STUBS` table `atelier
setup` writes from, so a new stub is diffed with no second list to update: shell
stubs as a set of non-comment lines, `~/.gitconfig` as `git config --list` keys,
the Claude override as `key.path=value` pairs. Covered by
`atelier/tests/local_diff.rs`.

**Claude Code settings are a special case.** Claude Code has no user-scope
`settings.local.json`, and it *rewrites* `~/.claude/settings.json` in place —
reordering keys and absolutising paths. So that file is **not symlinked**; it is
generated by `atelier claude-settings-sync` (run by `install.sh` after
`atelier setup`) as a 3-way merge:

```
drift     = live - snapshot      what the app changed since we last wrote
new local = local * drift        captured, so nothing machine-local is lost
live      = base * new local     base propagates where the app was silent
snapshot  = live                 recorded for next run
```

`dot_claude/settings.generated.json` is that snapshot. It exists solely to tell
"the app changed this" apart from "the base changed under me" — without it, a
base edit is indistinguishable from local drift, gets captured into the local
override, and the shared base can never propagate again. Both the local override
and the snapshot are gitignored.

`-` keeps every leaf of live that the snapshot lacks or disagrees with, and `*`
merges objects recursively while arrays and scalars replace wholesale — so a
captured array carries the base's own entries, and a key the app *deletes* is
not captured and comes back from the base. The cases, key reordering and
absolutised paths included, are `atelier/tests/claude_settings.rs`.

Run `atelier claude-settings-sync --dry-run` to preview what would be captured.

## Atelier

`atelier/` is a Rust binary crate (edition 2021, clap derive) that takes over
the tmux bash scripts one ticket at a time — see `docs/atelier/spec.md` and
`docs/atelier/tickets/`.

- **A feature is a module plus one line.** It lives in `src/<feature>.rs` (or
  `src/<feature>/`), exposes `pub enum Command` deriving `clap::Subcommand` with
  `pub fn run(self, tmux: &Tmux) -> Result<()>`, and is registered by one line in
  the `features!` list in `main.rs`, which declares the module and the
  subcommand. Shared modules (`tmux`, `session`, `git`, `shell`, `fzf`, `opener`) are plain `mod` lines.
  Keep `main.rs` and `Cargo.toml` small; they are the files every branch touches.
  A module whose commands sit at the top level (`atelier daemon`, `atelier
  status`) goes after `; top level:` in the same list and is flattened.
- **tmux is reached only through `tmux::Tmux`**, which always targets an explicit
  socket: `--socket`/`-S`, else the one in `$TMUX`. tmux sets `$TMUX` for `#()`
  jobs and `run-shell`, so the default is right when tmux calls atelier.
  The binary is the first `tmux` on `PATH`, else in `/opt/homebrew/bin`,
  `/usr/local/bin`, Linuxbrew's prefix, `/usr/bin` or `/bin`: hooks such as
  Claude Code's can run with a `PATH` that lacks the Homebrew prefix.
- **Every picker builds fzf through `fzf::picker`**, which carries the shared
  flags, colours and the `j`/`k`/`q` + `i` search mode, and calls back into
  atelier through `fzf::atelier` (the binary plus `--socket`). `fzf::leave_search`
  is the `esc` bind for pickers that restore their prompt without a callback.
- **Session identity comes from `session::resolve`, for every consumer**:
  `@grove_project`, else the basename of the main worktree (first entry of
  `git worktree list --porcelain -z`) of `#{session_path}`, else the session name.
  Its `root` (`@grove_root`, else that main worktree, else none) is what "the
  same project" means; a session with no root belongs to no project. Its
  `checkout` (root or worktree) is `@grove_worktree` being empty or not, else
  git's, else none.
- **The session picker is `atelier sessions pick`**, run by `Prefix + Space`
  and `Prefix + s` inside `display-popup -E`. It execs fzf with rows of
  `<session id><TAB><label>` (`--with-nth=2..`), and every bind calls back into
  `atelier sessions <rows|toggle|header|escape|switch|preview|next|prev|kill>`,
  which is what `tests/sessions.rs` drives. The scope lives in
  `@atelier_sessions_scope` on the session the picker was opened from, reset to
  `project` on every open. tmux 3.4 does not expand formats in
  `display-popup`'s shell command, so `pick` resolves the session and client
  itself rather than taking `#{session_id}` from the binding. Needs fzf ≥ 0.45
  (`transform`).
- **The preview is the highlighted session's window list plus the tail of its
  current window**, because fzf previews cannot be focused; `h`/`l` run
  `previous-window`/`next-window` on that *other* session (no client needed),
  so Enter lands on the window being watched. The capture comes at the source
  pane's geometry, bigger than the popup both ways: width is `nowrap` (a
  full-screen TUI cannot reflow, and a cut right edge reads where wrapping does
  not), height takes the tail sized from `$FZF_PREVIEW_LINES`, since fzf clips
  the bottom, which is the prompt or the error. Trailing lines that are blank
  once SGR is stripped are dropped first. `ctrl-x` is a `transform` that kills
  and prints `reload(rows)`.
- **`display-message -p` returns one field per call.** It prints control
  characters as octal and newlines as `_`, so fields cannot be joined with a
  delimiter — except numeric fields ahead of a single free-text one, split with
  `splitn`, as the session picker reads its rows; and with an unknown `-t` it exits 0 with empty output, so emptiness
  is the not-found signal. `git -C ""` runs in the cwd — never pass an empty path.
- **Tests drive the built binary against a private tmux server.**
  `tests/common/mod.rs` has `TmuxServer::start()`: `tmux -L
  atelier-test-<pid>-<n> -f /dev/null` with `exit-empty off`, killed on drop,
  one per test so tests run in parallel. `atelier(..)` passes `--socket`,
  `atelier_inside(..)` sets `$TMUX` instead. Assert on what tmux or the binary
  shows, never on internals; pure render functions are the only other seam.
- **The daemon (`src/daemon/`) is one per tmux socket.** `dot_tmux.conf` runs
  `atelier daemon --ensure` through `run-shell -b` at load and from a
  `session-created` hook (the load-time run can race the first session).
  `--ensure` returns at once when the lock is held, else spawns `atelier daemon`
  detached; the daemon itself takes the lock, so racing ensures still leave one.
  Lock, socket and log live in `$XDG_RUNTIME_DIR/atelier/` (else
  `$TMPDIR/atelier-<uid>/`) as `<fnv1a of the socket path>.{lock,sock,log}`.
  `atelier status` asks the daemon over that socket (JSON lines) and reads tmux
  directly when nothing answers; its first line says which.
- **Its only link to tmux is `tmux -C attach-session -f no-output,ignore-size`.**
  tmux 3.4 has no session-less control client — one started with no session
  prints `%exit` at once — so the daemon attaches to an existing session and
  never creates one. With `detach-on-destroy on` (tmux's default, not this
  config's) killing that session sends `%exit` while the server lives on, so
  after `%exit` the daemon reattaches if any session is left and exits
  otherwise; the `session-created` hook brings it back. Being attached, it is a
  client like any other in `list-clients`, `#{session_attached}` and the
  `client-*` hooks: every "is someone looking" check must skip
  `#{client_control_mode}` = 1. State is rebuilt from `list-sessions`,
  `list-windows -a`, `list-clients` on structural notifications; renames and
  window closes are applied in place. Replies are paired with requests in
  order, `%begin`/`%end` by command number, because a block's lines are not
  escaped and a window named `%end 1 1 1` is legal. Control mode prints a tab in
  format output as `_`, so fields are space-separated with the free-text one
  last.
- **The daemon pushes the bar** (`src/daemon/bar.rs`) for every session a
  non-control client is on, at the width of its most recently active one
  (`#{client_activity}`) — the option is per session, the width per client, so
  one has to win. It re-renders after any structural notification, a rename, a
  `%message atelier:client-resized` and a battery reading that changed (read
  once a minute), and only sends `set-option` when a block changed. Markers
  need nothing: they live in the window formats, which tmux evaluates itself.
  Session fields come over the control connection as `#{q:…}`-quoted words and
  go through `session::identify`, the resolver behind `session::resolve`.
- **Repo counts follow git through a file watcher, never a timer**
  (`src/daemon/repos.rs`, `notify`). Every repo some session's path is in —
  attached or not — is watched; it is dropped with the last such session. The
  repo is found on disk, not by asking git: a `.git` directory, or a `.git`
  file whose `gitdir:` is `<common>/worktrees/<name>` plus that dir's
  `commondir`. Watched: the worktree's own git dir (`index`, `HEAD`), the common
  dir (`HEAD`, `packed-refs`) and `refs/` recursively, and **the directory of
  every tracked file** — an unstaged edit touches nothing under `.git`. Worktree
  events count only for paths in the `ls-files` set, so untracked and ignored
  files never start git; the set is re-listed when the index changes. On
  FSEvents the worktree root is watched recursively instead, filtered the same
  way. `*.lock` and access events are ignored (git's own reads would otherwise
  loop). Events settle for 200 ms, then the repo's counts are dropped and
  recomputed lazily at the next push, only for sessions a client is on. The
  session identity (which runs `git worktree list` without grove options) is
  cached by session fields plus the repo found. So nothing forks git while the
  repo is untouched: `tests/bar_daemon.rs` puts a logging `git` first on the
  daemon's `PATH` and asserts switches, resizes, untracked and ignored writes
  start none. This relies on `diff-index`/`ls-files` under
  `--no-optional-locks` never writing the index.
- **Control mode has no resize notification for other clients.**
  `%layout-change` only covers windows of the daemon's own session, and
  subscriptions are evaluated against the daemon's client. So on attach the
  daemon sets `@bar_daemon` to its client name and installs
  `client-resized[73]` → `display-message -c <itself> atelier:client-resized`,
  which arrives as a bare `%message` (an `if-shell` wrapper would wrap it in a
  `%begin`/`%end` block instead). `client-detached[73]` unsets `@bar_daemon` and
  that hook when the detaching client is the daemon, which is what drops the bar
  to its fallback when the daemon is killed. Display-message's `-c` is not
  format-expanded, which is why the name is written into the hook.
- **Automatic rename is lazy.** tmux re-evaluates `automatic-rename` only when
  its event loop wakes, so a fresh window can read `tmux` (the forked server,
  before `exec`) until some unrelated command or output wakes it, and only then
  does `%window-renamed` arrive. Daemon tests turn `automatic-rename` off.
  Parser fixtures in `tests/fixtures/control/` are real tmux 3.4 transcripts;
  `record.sh` there re-records them.
- **Installed into `~/.local/bin/atelier`** by `install.sh` (`cargo install
  --locked --root ~/.local`, target dir `atelier/target` so a re-run is
  incremental; rustup with `--no-modify-path` when cargo is missing, and
  `dot_zshrc` puts `~/.cargo/bin` on `PATH`). tmux and scripts call it by that
  absolute path, and every caller must still work when it is absent. The install
  itself is the exception: `install.sh` stops if the build fails, since
  `atelier setup` is what links everything.
- **A feature with flags and no subcommands** (`setup`, `local-diff`,
  `claude-settings-sync`, `doctor`) derives
  `clap::Args` and calls `crate::flags_only!(Command)`, which implements
  `clap::Subcommand` by delegating to the args and clearing
  `subcommand_required`, so it still registers with one `features!` line.
- **`atelier doctor` turns what this file asks you to remember into checks**,
  each a small function in a table in `src/doctor.rs` printing `ok`, `FAIL`
  with a one-line `fix:`, `skip` or `look`; any `FAIL` exits non-zero. It
  checks terminfo for `$TERM` and every attached terminal (fix: `infocmp -x
  <term> | ssh <host> tic -x -`), tmux ≥ 3.3 (`allow-passthrough` and
  `pane-border-indicators` are the newest options `dot_tmux.conf` sets),
  `extended-keys`, duplicated `terminal-features`, fzf ≥ 0.45,
  `~/.local/bin/atelier`, pending `atelier setup` changes, the keyboard
  layout bundle (macOS only; selecting it stays manual), the daemon of this
  tmux server, every service of `services.toml` (and on launchd a writable
  `~/Library/LaunchAgents`), and linger on systemd. Checks that need a tmux
  server are skipped outside one. **The Nerd Font is `look`, never `ok`**: no
  terminal reports which font draws a glyph, so it prints U+E0B6/U+E0B4 for a
  human to judge and does not affect the exit code. `tests/doctor.rs` drives it
  against a private tmux server with fake `infocmp`, `fzf`, `systemctl`,
  `loginctl` and `launchctl` alone on `PATH`.
- **CI** (`.github/workflows/atelier.yml`): `cargo fmt --check`, `cargo clippy
  --all-targets -- -D warnings` and `cargo test` on Linux and macOS, plus
  shellcheck on `install.sh` and every executable shell script in `scripts/`.
  CI installs a pinned fzf release (Homebrew's on macOS), since distro packages
  are older than 0.45; the fzf-driven tests skip without a recent fzf locally
  but fail under `CI`. Run the same three cargo commands in `atelier/` before
  pushing.

## Tmux

**There is no session persistence — no tmux-resurrect, no tmux-continuum, no
tpm — and that is deliberate.** `grove` (the git-worktree manager at
`~/Developer/perso/grove-ai`, see the `grove` skill) is the session factory: a
session is derived from a worktree, so it is reproducible on demand from the
repo plus `.grove/config.yaml`. Restoring a save-file would reconstruct whatever
happened to be open at the last checkpoint; recreating from a worktree
reconstructs what the project *is*. Do not add a persistence plugin.

**`terminal-features`, not `terminal-overrides`, and never with a bare `set -a`.**
`set -a` on an array option appends, so a reload — `Prefix + r` — grows the array
every time; the live server had accumulated 52 duplicated `terminal-overrides`
entries this way. `set -gu` first resets the option to its default array (which
carries the stock `xterm*:clipboard:ccolour:cstyle:focus:title`, needed for OSC 52
and focus events), so the `set -gu` + `set -as` pair is idempotent. `atelier
doctor` fails on a duplicated entry.

**No terminal app is assumed.** Entries are keyed on the client's TERM, outside
tmux: `xterm-ghostty`, `xterm-kitty` and `wezterm` get `RGB:usstyle:sync` —
`usstyle` is what makes nvim's LSP undercurl render as a curl rather than a plain
underline — and plain `xterm-256color` gets `RGB` alone, since that TERM is also
what Terminal.app and a bare xterm report, and the entry can only promise what the
least capable of them supports.
iTerm2 has no entry because it cannot have one: it reports `xterm-256color`, and
tmux recognises it from its XTVERSION reply and applies `RGB`, `usstyle` and
`sync` itself. WezTerm reports `wezterm` only when its `term` option is set to it
and the terminfo entry is installed; on its default `xterm-256color` it gets the
plain entry. Inside tmux, `default-terminal` stays `tmux-256color`.

**`detach-on-destroy off`**: killing the last window of a session switches to
another session instead of ejecting the client out of tmux.

**`Prefix + 1..9` depends on a keyboard layout that macOS, not tmux, provides.**
`keyboard/FR-AZERTY-num.bundle` is an AZERTY layout with an unshifted,
QWERTY-order number row, **copied** to `~/Library/Keyboard Layouts/` by
`install.sh` (`copy_bundle()`, not a link — macOS's input-source daemon does
not reliably follow a symlink there). Without it the digit bindings need Shift
and the muscle memory
breaks silently. Installing is automated, *selecting* is not: see README. The
bundle's 322 KB `.icns` is deliberately not tracked — it is only the menu-bar
glyph, macOS falls back to a generic icon, and it was 87 % of the payload.

`next-layout` is on `Prefix + L` (tmux's own `Prefix + Space` went to the session
picker). It is the only keyboard way to rebalance a split here: there are no
resize bindings, on purpose — resizing is done by dragging the pane border.

## Per-project tmux tasks

`Prefix + e` opens a task picker (`atelier tasks pick`) over `<project>/.tmux/`.
Build, run and debug loops live there rather than in Neovim, so they can be
driven from any window of the session.

`.tmux/` is ignored globally via `git/ignore`, so tasks stay untracked in any
repo — including ones we don't own — without editing that project's
`.gitignore`.

**A task is any executable file at depth 1 or 2 under `.tmux/`.** The executable
bit is the only filter, which is what lets `.tmux/lib/common.sh` stay out of the
picker with no naming convention and no ignore list. Subfolders become groups
(`Tab` cycles them); depth-1 files are ungrouped.

```bash
#!/usr/bin/env bash
# task: assemble the debug APK and install it     <- picker description
# tmux: window    window | split-down | split-right | popup | detach
```

Splits take an optional size (`split-right 40%`, `split-down 15`, default 30%;
`split` alone means `split-down`) and split the session's active pane. They are
named by direction, not `-v`/`-h`, because those are inverted between tmux and
vim. `popup` reuses the picker's own popup: tmux allows one popup per client,
and a nested `display-popup` silently does nothing while still exiting 0. Run
outside the picker (`atelier tasks run`), a popup task opens one with
`display-popup`, which is spawned and not waited for: from the command line,
`display-popup` only returns once the popup closes.

Read from the first 20 lines only. `window` and `detach` reuse a window named
after the task (`android/build` → `android-build`), respawning it rather than
piling up duplicates. `detach` runs unselected, and is re-run with
`respawn-pane`, not `respawn-window`: `respawn-window` has no `-d` and makes the
window current.

Every task runs with cwd at the project root and `TMUX_TASK_ROOT` /
`TMUX_TASK_NAME` set, resolved from `#{session_path}` — the session's working
directory, **not** the pane's. That is what makes the picker behave identically
from a pane three directories deep. Ordering is most-recently-run first, cached
per project under `$TMPDIR`.

Every placement runs the task under `atelier tasks exec`, which tmux starts
directly as the pane's command (argv, no shell), so it publishes the marker,
rings and waits for a key itself; a popup picked in the picker runs it in the
picker's own process. The picker, the rows and its fzf callbacks (`atelier tasks
list|advance|prompt|header`) and every placement are covered by
`atelier/tests/tasks.rs`; the popup test drives the picker inside a real popup
with a stub `fzf` that picks the first row.

**Task completion is signalled by `@task_status`**, a per-window user option the
wrapper sets to `running` / `ok` / `fail`; the `window-status-*` formats render
it as `●` / `✓` / `✗`. It is set on the *window*, so it survives
`respawn-window`, which is why every run resets it to `running` first —
otherwise the previous run's `✗` would sit over a task that is now succeeding.
The window is resolved from `$TMUX_PANE`, not the session's active window, or a
`detach` task would mark whichever tab you happen to be looking at.

Only `window` and `detach` are marked. A split or popup shares the window you
are working in, where a `✗` would be ambiguous — and the output is right in
front of you anyway.

Those same two placements also **ring** on completion: a `\a` bell, picked up by
`monitor-bell on`, then passed on to the terminal, which decides whether that
means a dock bounce or a badge. It is skipped when the task's window is the
active window of a session a non-control-mode client is attached to — ringing
about output the user is staring at is noise, and a control-mode client (the
daemon) looks at nothing. Tasks ring and nothing else: the spec keeps desktop
notifications for Claude's `waiting` only.

`monitor-activity` is deliberately **off**. It flags a window on any output at
all, so Neovim and Claude Code kept it permanently lit and it carried no
information. `monitor-bell` stays on: a BEL is rare enough to mean something.
Claude Code's own state is a window marker instead — see below.

**Both pickers always open, even with nothing to list.** `Prefix + e` used to
gate on a `--check` of the task list via `if-shell` and `Prefix + Space` bailed out with
`display-message` when there was no other session; both now render a placeholder
row instead — `(no executable task in .tmux/)`, `(no other session)`. The answer
is the same either way, and putting it in the popup puts it where the eye
already went. fzf has no non-selectable row, so the placeholder is a real entry
and every action on it — Enter, the preview, `ctrl-e`, `ctrl-x` — is a no-op.
The session picker needed this regardless: `Tab` is the only way out to the
other projects, and it cannot be pressed in a popup that never opened.

## Claude Code window marker

`atelier hook claude` publishes `@claude_status` on the window Claude Code is
running in — `waiting` on the `Notification` event, `done` on `Stop`, cleared on
`UserPromptSubmit` — and the window list renders it as a yellow or green `✻`.
All three hook entries in `dot_claude/settings.json` are the same command; the
event comes from `hook_event_name` in the payload on stdin rather than an
argument, and the window from `$TMUX_PANE`, not the session's current window.

The command always ends in `|| true`. Claude Code treats a hook's exit status 2
as *blocking* — on `UserPromptSubmit` it discards the prompt — and 2 is also
what clap exits with on an unknown subcommand, so an atelier binary older than
the hook would otherwise eat every prompt. `atelier hook claude` itself exits 0
whatever goes wrong; a missing binary is the same as no marker.

**It replaced a `terminal-notifier` banner, which failed for reasons no amount of
fixing addressed**: with several sessions running there was no telling which
instance had fired, so the first one visible got opened and corrected afterwards;
the banner was not clickable back to the right window; and it was in the wrong
place — a notification asks you to leave the terminal to learn something about
the terminal.

**The marker is the only signal: there is no bell.** The hook used to send one so
that ghostty's `bell-features` gave a dock badge when the terminal was not
frontmost, and Claude Code's own notification channel — unset, i.e. `auto`, which
resolves to `terminal_bell` outside iTerm — sent a second one. Two
indistinguishable beeps, neither of them locating anything, and only one of them
gated on whether the window was already in front. A beep that cannot be
attributed to a window is a beep that stops being trusted, and an untrusted
signal is pure noise, so both are gone: the `printf '\a'` from the hook and
`preferredNotifChannel: notifications_disabled` in `dot_claude/settings.json`.
Nothing signals from outside the terminal any more — that is the accepted cost.
`monitor-bell` stays on for tasks, which do still bell.

**It reuses the `@task_status` slot rather than adding a second marker.** That is
the whole reason this was cheap: the slot is already 3 columns wide with its
width parity solved across state and focus, and a second independent marker would
have meant redoing that accounting over a much larger matrix. Task state takes
precedence where both are set, which in practice does not happen — Claude runs in
its own window. `✻` is U+273B, East Asian Width **N** like `✓` and `✗`, so it
occupies the same single cell.

The states reuse the task palette because they mean the same things — yellow is
"needs you", green is "finished" — which keeps the rule that the window list
carries exactly two signals, blue for focus and the marker for state. Only the
glyph says which subsystem is talking.

A marker is not set when its window is the current window of an attached
client, and `after-select-window` in `dot_tmux.conf` clears it, so going to look
is what dismisses it. Without that skip the marker would appear on the window
being watched with nothing left to clear it, since selecting it has already
happened. Control-mode clients (`tmux -C`, as iTerm2 and the planned daemon
attach) do not count as looking. All of this is covered by
`atelier/tests/hook_claude.rs`.

## One-shot command popup

`Prefix + Enter` opens a popup that takes **one** command and then goes away.
Rooted on `#{session_path}` like tasks are, never the pane, so `grove` or a
`g sync` act on the right repo from any window — including one running Neovim,
which is the case the shell alias cannot serve.

It runs a real interactive zsh, not a `read` prompt: the point is to *type* a
command, so aliases (`g`), completion and history all have to be there. The
private `ZDOTDIR` at `tmux/oneshot/` sources `~/.zshrc` and adds two hooks.
`preexec` marks that a command actually ran — so a bare Enter at the prompt does
not spend the shot — and `precmd` then prints `✓` or `✗ exit N`, waits for a
single keypress, and exits.

Setting `ZDOTDIR` has a sharp edge worth knowing: macOS's `/etc/zshrc` assigns
`HISTFILE=${ZDOTDIR:-$HOME}/.zsh_history` unconditionally, and oh-my-zsh derives
`ZSH_COMPDUMP` the same way — so zsh writes its history and completion dump into
*this tracked directory*, and a `.zsh_history` from the popup was staged for
commit once before being caught. Both are pinned back to `$HOME` in the file
(`ZSH_COMPDUMP` before the source, since oh-my-zsh only fills in a default;
`HISTFILE` after, since `/etc/zshrc` would otherwise overwrite it), with a
`.gitignore` as a backstop. Pinning also means the popup shares the real
history, so up-arrow reaches what was typed in an ordinary shell.

**Both outcomes wait for a key.** An earlier version closed itself on a zero
exit status, which reads well until a successful `git log` or `git status`
vanishes before it can be read. The keypress is what replaces `Ctrl-D`: any key
rather than a chord, and no opportunity to type a second command into a popup
meant for one.

Being able to run an arbitrary command in a popup is why grove needs no entry in
the task picker, and why the picker has no free-form mode. Note that tmux allows
one popup per client, so this cannot be opened from inside the task picker.

## Project jump

`project <name> <path> [tab]` in `dot_zshrc` defines a function named `<name>`
that enters a project in its default state: `cd`, rename the ghostty tab, then
`grove attach` on the main worktree's branch. Typing `biogroup` lands on the
project's main session from anywhere.

The registrations themselves are **machine-local** — they carry absolute paths
this repo cannot ship — so they live in `~/.zshrc` below the load line:

```sh
project biogroup ~/Developer/theodo/biogroup "Biogroup"
project grove    ~/Developer/perso/grove-ai  "Grove"
```

`local-diff` therefore reports every registration as a local addition, on every
run, and that is accepted rather than filtered. Its premise is that each finding
is a decision — and for a `project` line the decision is always "leave it local",
so the lines are pure noise that grows one per project. Teaching it to skip them
was rejected anyway: it is currently a plain line-set diff with no knowledge of
zsh syntax, and a diff tool that silently drops lines stops being trustworthy
about the ones it does report.

Three fields, and nothing per-project anywhere else. A registration whose path
does not exist is skipped silently, so the same block is safe to carry to a
machine that has not cloned everything; one whose name already resolves to a
command is skipped *loudly*, because a project called `make` or `ls` would
otherwise shadow it with no clue as to why. Re-registering is allowed — the
`_project_paths` membership test runs before the shadow check — so
`source ~/.zshrc` does not start warning about the functions it just defined.

**The branch is derived, never hardcoded to `main`.** `grove ls` names the root
checkout after its branch, and not every repo here is on `main`. The main
worktree is the first entry of `git worktree list --porcelain` (which is its
definition, and unlike `awk '{print $2}'` on that line, `${root#worktree }`
survives a path with a space in it); a detached HEAD there falls back to
`origin/HEAD`. `grove attach` already creates the session if it is missing and
switches the client instead of nesting when called from inside tmux, so both
cold start and project switching are the same call. Everything after the `cd` is
guarded on `grove` being on `PATH` and the directory being a git repo — without
either, the `cd` still happens, which is the half that always works.

**The tab name is set by the jump and never touched again.** tmux runs with
`set-titles off` (its default; `dot_tmux.conf` never sets it), so tmux emits no
title of its own and nothing overwrites what the function wrote — the same
reason a manual ghostty rename sticks. Inside tmux the OSC 2 sequence has to go
through tmux's passthrough wrapper, which requires every ESC of the inner
sequence doubled and depends on `allow-passthrough on`, already set for other
reasons.

The alternative — `set-titles on` with a `set-titles-string` resolving the
project from `#{session_path}` — was rejected as more machinery for a property
that only has to be right at cold start. Its one advantage is that the title
would follow the *attached session*: as it stands, jumping to `biogroup` and then
switching to another session with `Prefix + Space` leaves the tab reading
"Biogroup". That is already true of a hand-renamed tab, and the status bar's left
block is the thing that stays truthful.

## cc-tap (Claude Code dashboard + inspector proxy)

[cc-tap](https://github.com/theodo-group/cc-tap) runs permanently as three
services declared in `services.toml` (see "Services" below), all driven by
`scripts/cc-tap-service`:

| service | runs | when |
| ----- | ---- | ---- |
| `com.theodo.cc-tap.dashboard` | dashboard on `127.0.0.1:3000` | login, kept alive |
| `com.theodo.cc-tap.proxy`     | inspector proxy on `127.0.0.1:8089` | login, kept alive |
| `com.theodo.cc-tap.update`    | `npm install cc-tap@latest` | see below |

`cl` in `dot_zshrc` routes through the proxy when `:8089` accepts a connection
and falls back to plain `claude` otherwise; `--no-proxy` anywhere in the
arguments forces plain and is stripped before `claude` sees it. There is no
separate proxy alias any more — one command, and the proxy being down never
blocks a session.

**A service manager, not Docker.** `proxy/server.js` hardcodes `listen(PORT, '127.0.0.1')`,
so inside a container it is unreachable through a published port without
patching the package. It also reads `~/.claude` and writes `~/.cc-lens`, so a
container would buy no isolation.

**The services bypass the `cc-tap` CLI** and run the standalone `server.js` and
`proxy/server.js` directly: the CLI opens a browser tab on every start, and in
the dashboard the proxy is a detached child spawned from the Live Capture
button, which dies with every restart. The proxy agent writes
`~/.cc-lens/proxy.json` itself because that file is the only way the dashboard
knows a proxy exists; without it, Start spawns a second one on `:8090`. The
flip side is that the dashboard's Stop button kills the proxy and the service
manager brings it back ten seconds later — to really stop capture, `launchctl
bootout gui/$(id -u)/com.theodo.cc-tap.proxy` (macOS) or `systemctl --user stop
com.theodo.cc-tap.proxy` (Linux).

**Node is fnm's `default` alias**, not Homebrew's `node`: Homebrew's is only on
a machine as a dependency of something else (it is not in the `Brewfile`),
whereas fnm's default is provisioned by `install.sh` and lives at a stable path.
cc-tap needs Node ≥ 24.

**Updates: once per weekday, from 08:00, restarting only on a new version.** The
update agent fires at load, every 30 minutes, and at 08:00 Mon–Fri; the script
decides. It no-ops on weekends, before 08:00, and once today's stamp
(`~/.local/share/cc-tap/last-update`) is written — the stamp is only written on
a successful install, so being offline just means the next tick retries. That
covers the three ways 08:00 gets missed: asleep (launchd runs a missed calendar
event on wake; systemd's `Persistent=true` does the same), powered off
(`RunAtLoad`; `OnActiveSec=0`), and no network. Restarting is skipped when the
version did not change because restarting the proxy drops every in-flight
request of every Claude session routed through it. The script restarts through
`launchctl kickstart -k` on macOS and `systemctl --user restart` elsewhere, and
logs to `~/Library/Logs/cc-tap/` or `${XDG_STATE_HOME:-~/.local/state}/cc-tap/`.

## Services

`services.toml` at the repo root declares every background service once — a
label, a description, the program relative to `$HOME` with its arguments, an
optional log file name, and an optional `schedule` (`days`, `at`, `every`
seconds). No `schedule` means keep-alive. `atelier service install|list|uninstall`
turns it into launchd agents on macOS and systemd user units on Linux
(`--system` overrides the guess); `install.sh` runs `install`.

- **launchd**: one plist per service in `~/Library/LaunchAgents`, **copied, not
  linked**, the same treatment as the keyboard bundle. launchd expands neither
  `~` nor `$HOME`, so each runs `/bin/sh -c 'exec "$HOME/…"'`, with the log
  redirected to `~/Library/Logs/<log>` when the description names one.
  Keep-alive is `RunAtLoad` + `KeepAlive` + `ThrottleInterval 10`; a schedule is
  `RunAtLoad` + `StartCalendarInterval` (+ `StartInterval` for `every`).
- **systemd**: `<label>.service` in `${XDG_CONFIG_HOME:-~/.config}/systemd/user`,
  `ExecStart=%h/…` (systemd's own home specifier, so no shell), `Restart=always`
  and `RestartSec=10` for keep-alive. A schedule adds `<label>.timer` with
  `OnCalendar=Mon..Fri 08:00`, `Persistent=true`, `OnActiveSec=0` for
  `RunAtLoad` and `OnUnitActiveSec` for `every`; the timer is what gets enabled.
  Output goes to the journal (`journalctl --user -u <label>`).

**A file is written only when its content changed, and a service is reloaded
only then** — so re-running `install.sh` does not bounce cc-tap's proxy. On
launchd, an unchanged and loaded agent is only `kickstart`ed (without `-k`),
which starts a stopped one and is a no-op on a running one; a changed or
unloaded one is `bootout`ed — waiting up to 5 s for it to go, since a
`bootstrap` issued while it lingers fails with `5: Input/output error` — then
written, `bootstrap`ped and kickstarted, because a fresh bootstrap can sit at
`pended nondemand spawn = speculative` for minutes. On systemd, any change
triggers one `daemon-reload`, then `enable` + `restart` of what changed and
`enable --now` of the rest. For `update` either path means one extra run of the
script, which gates itself.

`~/Library/LaunchAgents` can end up owned by root — the Pulse Secure installer
did it on this machine — which makes every write fail with `EACCES`. `install`
probes the folder first and fails with the `sudo chown` that fixes it;
`install.sh` turns that into a warning. On Linux, `install` runs `loginctl
enable-linger` unless `Linger=yes` already, so services survive SSH logout.

`tests/service.rs` snapshots every generated file for both systems (the launchd
snapshots started as the hand-written plists they replaced, byte for byte) and
drives `install`/`list`/`uninstall` against a temporary `$HOME` with fake
`launchctl`, `systemctl` and `loginctl` on `PATH` that log their arguments. The
unwritable-folder test skips itself as root, where permissions are not enforced.

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

`nvim/` is a plain directory in this repo — commit changes to it like anything
else. It was a git submodule pointing at a `kickstart.nvim` fork until that fork
had diverged far enough that upstream merges were no longer realistic (the
inherited stylua workflow was still gated on `github.repository ==
'nvim-lua/kickstart.nvim'` and had never once run). The submodule was charging a
commit-plus-bump dance per change and a clone that broke whenever it had not been
pushed, so it was folded in with `git subtree`.

The fork's 444 commits came across and are reachable, but pre-fold commits carry
un-prefixed paths, so `git log -- nvim/<file>` stops at the merge. Use
`git log <merge>^2` to walk the old history. Upstream kickstart is no longer
tracked; do not merge from it.

Lua formatting is checked by `.github/workflows/stylua.yml` at the **repo root** —
a workflow under `nvim/.github/` would silently never run.

**Structure:** `nvim/init.lua` loads `config.options`, `config.keymaps`, `config.autocmds`, `config.lazy` (in that order). All plugins are managed by lazy.nvim in `lua/config/lazy.lua`. Custom plugins go in `lua/custom/plugins/`, kickstart extras in `lua/kickstart/plugins/`.

**LSP keymaps** are consolidated in a single `LspAttach` autocommand inside the Telescope config section of `lazy.lua`. The lspconfig `LspAttach` handles only document highlight and inlay hints. `grn`/`gra` are Neovim 0.11+ built-in defaults (not explicitly mapped).

**Formatting:** Stylua with 2-space indent, single quotes (see `nvim/.stylua.toml`).
It is in the `Brewfile` so `stylua --check nvim/` can be run before pushing —
without that the CI gate could only ever fail after the fact.

**Smoke test:** `nvim/scripts/test-config.sh` runs headless Neovim validation.

## Kotlin and Swift (KMP + native mobile)

**Kotlin gets no language server, on purpose.** `kotlin-language-server` is
effectively unmaintained and does not model multiplatform source sets or
`expect`/`actual`; JetBrains' `kotlin-lsp` is pre-alpha and JVM-only. Both
produce diagnostics that are wrong often enough to be worse than none. Kotlin in
Neovim is treesitter (highlight, indent, text objects, symbol picker), ripgrep
and ktlint. Completion, type-aware rename and debugging happen in Android
Studio; the same split puts iOS debugging in Xcode. **Do not add a Kotlin LSP,
and do not add DAP for Kotlin or Swift.**

Swift is the exception: `sourcekit-lsp` is Apple's own and already on disk. It is
added to `servers` in `lsp.lua` **after** `ensure_installed` is computed, because
mason has no package for it, and guarded on `executable('sourcekit-lsp')` so it
stays inert without Xcode. It is accurate for SwiftPM packages and weaker on an
`.xcodeproj` — that would need a `buildServer.json` from `xcode-build-server`,
which is deliberately not installed.

**Formatters are not in the `Brewfile`.** `ktlint` comes from mason (brew's
formula depends on `openjdk`, i.e. a second JDK next to the one `JAVA_HOME`
already points at) and doubles as the Kotlin linter — it is the only entry in
`nvim-lint`'s `linters_by_ft`. `swift-format` ships inside Xcode's toolchain and,
unlike `sourcekit-lsp`, is **not** shimmed into `/usr/bin`, so conform invokes it
as `xcrun swift-format`. Net effect: nothing new is installed on a machine that
does no mobile work.

`detekt` is intentionally absent: it is a Gradle plugin driven by project-level
config, so it belongs in the project's build, not here.

Gradle and Xcode generate very large trees, so the snacks explorer — which runs
with `hidden`+`ignored` on and therefore gets no `.gitignore` filtering — carries
an explicit exclude list (`build`, `.gradle`, `.kotlin`, `DerivedData`, `Pods`,
`xcuserdata`, …). Treesitter installs `kotlin`, `swift`, `java`, `groovy`, `xml`
and `properties` (filetype `jproperties`, for `gradle.properties` and
`local.properties`).

Opening a whole project in Studio or Xcode is a shell/tmux concern, not an editor
one: `studio` in `dot_zshrc` (`open -a`, so the IDE outlives the shell) and
`xed`, which Xcode already provides. Per-project variants belong in that
project's `.tmux/` tasks.

## Key Integrations

- **Tmux ↔ Neovim**: `vim-tmux-navigator` for Ctrl+h/j/k/l pane navigation. Tmux has `focus-events on` for Neovim autoread and gitsigns refresh. The same four keys are re-bound in `copy-mode-vi`, where they otherwise fall through to tmux defaults — `C-h` was a duplicate `cursor-left` and `C-j` was `copy-pipe-and-cancel`, i.e. it yanked and exited the mode.
- **Git ↔ Delta**: `dot_gitconfig` includes `delta/themes.gitconfig` for diff rendering. Lazygit also uses delta with custom side-by-side/inline pagers.
- **Terminal ↔ Tmux**: `extended-keys on` decodes the terminal's CSI u sequence and `S-Enter` re-sends `\x1b[13;2u`, so Shift+Enter reaches Claude Code.
- **Tmux modes**: two one-shot modal tables, `Prefix → p` (pane) and `Prefix → t` (tab).

## Tmux status bar

`atelier bar left -t #{q:session_id} "#{client_width}"` renders the left segment
as two adjacent capsules — **project** and **root/wt** (`src/bar/left.rs`, a pure
function of the project, the checkout kind, the client width and the right
block's width; its snapshots cover every width tier at the root, in a worktree,
outside a repo and with a long name). Both come from `session::resolve`: root/wt
is `@grove_worktree` being empty or not, else whether `#{session_path}` is a
linked worktree (`--git-dir` ≠ `--git-common-dir`); outside a repo there is a
single grey pill. When atelier is missing or fails, the `#()` falls back to
echoing the session name. They are separate blocks on purpose: which repo you are in and which checkout of it you
are in are two different questions, and a flag glued onto the project name reads
as part of the name.

It does not parse the session name. Grove builds names as
`{prefix}{project}_{branch}_{key}` with a sanitizer that maps `/[.\s:/@]/` to
`_` and leaves existing `_` alone, so `grove_my_repo_main_f2d1` cannot be split
from the left, and its shape also matches a hand-named session like
`api_perf_beef`. A name also freezes the branch at session creation, so it
starts lying after the first `git checkout`. Nothing about the bar is
grove-specific: any session in any repo gets the same treatment, and a session
outside a repo falls back to its own name.

**The branch is deliberately not shown.** It is already visible in lazygit, in
the prompt and in nvim's own status line, and at 22 columns it was the single
widest field on the bar.

**The block pads out to match the right-hand one** so the window list starts in
the same place whatever session is attached. The padding sits *outside* the
capsules, not inside them: an internally padded pill leaves a visibly empty
capsule for a short name, which is exactly what made the previous 48-column
version read as a slab of dead space. The checkout segment stays `root`/`wt`
rather than the worktree's name because that name is nearly always a sanitized
copy of the branch, and it changed width on every switch.

The project pill hugs its name up to `MAX_PROJECT` cells and then truncates
with `…`. Wherever the block pads to match the right one, a longer name spends
padding, not layout, so that cap is a purely visual choice and there is room to
raise it.

`atelier bar right "#{session_path}" "#{client_width}" "#{client_key_table}"`
renders the *entire* right block — key table, repo state, battery, date, clock
(`src/bar/right.rs`, a pure function of the counts, the battery and the time;
its snapshots cover every width tier with and without counts and battery). Every
cell of it is still laid out in atelier, even the parts tmux fills in: the block
has to know its own total width (see below), and a piece it does not lay out is
a piece it cannot measure.

**Both `#()` calls are only the fallback.** The daemon renders `@bar_left` and
`@bar_right` per session and `dot_tmux.conf` shows them while the global
`@bar_daemon` is set, else runs the `#()` as before. The pushed right block
(`right::pushed`) stops short of three things tmux does better: the key-table
slot stays a tmux format (`#{p10:…client_key_table…}`), because control mode has
no key-table notification and a subscription would report the *daemon's* key
table; the date goes in as the literal `%a %d %b` (battery `%` doubled) and is
shown through `#{T:@bar_right}`, so it turns over at midnight with no timer; the
clock is `%H:%M` in the config, after it. `tests/bar_daemon.rs` checks the
pushed form plus those pieces equals what `atelier bar right` prints.

Repo state is `+412 −89 ↑2 ↓1` — **lines** changed against HEAD, then divergence
from the upstream. Two cheap calls, `diff-index --shortstat HEAD` and `rev-list
--left-right --count @{upstream}...HEAD`, rather than one `status --porcelain=v2
--branch`, which would cost a full worktree scan for the ahead/behind alone.
Untracked files contribute nothing: `--shortstat` only walks tracked content. It
is the plumbing `diff-index`, not `diff`: porcelain `git diff` refreshes and
rewrites a stat-dirty index even under `--no-optional-locks`, which collides
with an interactive git in the same repo (`tests/bar_right.rs` checks the index
is left alone). No upstream or a detached HEAD shows the line counts only; an
empty repo shows nothing.

The counts are padded on the *left*, so they grow away from the clock instead of
shoving it. Their width accounting charges the separator space where it is
emitted, not a flat +2 per part: a flat charge makes a segment whose first part
is `−` or `↑` come out one column narrow, which drifts the whole centred window
list by one — visible only when switching to a session that has no local edits
but is ahead or behind.

Widths are counted in terminal cells (`unicode-width`), so nothing depends on
`LANG` — tmux runs `#()` commands with the *server's* environment, and the bash
this replaced over-padded every glyph by two or three columns without a UTF-8
locale (`tests/bar_left.rs` and `tests/bar_right.rs` render under `C` and a
UTF-8 locale). The battery comes from the `starship-battery` crate (IOKit on
macOS, sysfs on Linux); on a machine with none the segment and its gap are
dropped and the right block is narrower.

Truncation happens inside atelier, never via `status-left-length`: tmux
truncates the *expanded* string, which by then contains `#[fg=...]` escapes, and
will happily cut one in half and print the remainder as literal text.

**Colour rule: the window list is greyscale; everything else gets one accent per
concept.** The list itself must stay neutral so its two signals read — blue is
"you are here" (active window pill, active pane border) and yellow/green/red are
task state (`●` `✓` `✗`, see below). An earlier version had a green
active-window block that made a green `✓` invisible. Outside the list, each
segment owns a hue and no hue is reused: mauve `#ca9ee6` project, peach
`#ef9f76` worktree (absent when at the root, where the chip is grey), flamingo
`#eebebe` repo state, teal `#81c8be` battery, red `#e78284` battery under 20 %.
None of them may be `#8caaee` or a task-state colour.

**Centring is only true while the two blocks are the same width.**
`status-justify centre` centres the list in the space *remaining* after
`status-left` and `status-right`, not in the terminal: measured on a 120-column
client, growing the right block by 36 columns moved the list 18 columns left —
exactly half. So `bar left` pads out to match, taking the number from
`right::width` in the same crate rather than a copy of the tier table, which
would drift.

Everything on the right is therefore fixed-width, including the key table slot,
which stays reserved at rest — letting it collapse would change the block width
every time a mode is entered and slide the list. For the same reason the date is
`%a %d %b` and never `%-d`, which would lose a column from the 1st to the 9th.

**Narrowing sheds whole segments** rather than crushing the list: date, then
battery, then the repo counts, leaving the clock. Before that, an 80-column
client rendered date and battery in full and dropped the *window list* entirely
— the one thing on the bar worth keeping. None of that engages above 120
columns. The left block's padding gives way earlier: it never leaves less than
36 columns between the two blocks, so below twice the right block plus 36 (154
columns with a battery, 134 without) the list drifts off centre instead of
being squeezed.

**A window occupies the same width whether or not it is active**: the active
format's two `` capsules are replaced by two plain spaces in the inactive one.
Without that, focusing a window widened it by a column and shoved every window
to its right. The `@task_status` marker is worth +3 columns on *both* sides, so
parity holds in all twelve combinations of state and focus.

**The task marker belongs to a window, visibly.** On the active window it is the
coloured *tail of the pill* — dark glyph on a yellow/green/red ground, with the
closing cap taking that colour — rather than a glyph outside the capsule, which
read as unattached. It cannot simply be drawn onto the blue pill: `#e5c890` on
`#8caaee` is the same low-contrast trap that already cost a green active-window
block. On inactive windows the marker sits one space after its own name and four
before the next, because centred between two windows there was no telling which
one it belonged to.

**A comma inside a `#[...]` must be escaped as `#,` when the style sits inside a
`#{?...}`** — otherwise it ends that branch of the conditional and the rest of
the style is silently dropped, leaving the branch rendering as if empty. So
`#[bg=#a6d189#,fg=#303446]`, not `#[bg=#a6d189,fg=#303446]`.

The right-hand segments are separated by spacing alone. An earlier version used
dim `·` bullets between them; at `#626880` they read as empty slots rather than
as separators.

**Shape carries as much as colour.** Only two things on the bar are filled
capsules — the project, always leftmost, and the active window, which moves.
Position tells them apart. The `` `` caps are U+E0B6 / U+E0B4; a Nerd Font is
required, which `ghostty/config` already pins. Note that these live in the
private-use area and some editors silently drop them on write — check with
`grep -c` after touching either file.

The palette is Catppuccin **Frappe**, matching ghostty. The bar background
(`#414559`) is one step *lighter* than the terminal (`#303446`) so it reads as
chrome on top rather than a hole punched in the window.

## Copying out of a pane

Two mechanisms, for two shapes of thing:

- **`Prefix + u`** (`atelier pick popup`, `atelier/src/pick.rs`) — fuzzy-pick a
  **token**: URL or file path. `Enter` opens (URL → the browser, file → the nvim
  in this session, or a new `nvim` window at the session root if there is none),
  `Ctrl-y` copies, `Ctrl-o` hands it to the OS opener (`open` on macOS,
  `xdg-open` elsewhere), `Ctrl-v` previews a markdown path (`atelier preview`)
  and does nothing on anything else.
- **`Prefix + v`** — copy-mode, then drag with the mouse. For a **region**.

`Prefix + v` matters because tmux's default `MouseDrag1Pane` only starts a
selection when `#{mouse_any_flag}` is unset — i.e. when the pane's application
has not asked for mouse events. Neovim holds it permanently and Claude Code
toggles it while rendering interactive UI, which is why dragging works
sometimes and not others. Inside copy-mode the `copy-mode-vi` table owns the
mouse unconditionally, and it respects pane borders (the terminal's own
selection — Shift+drag, Option+drag in iTerm2 — does not: it selects by screen
column, so a vertical split gives you both panes on every line).

The picker checks path candidates against the filesystem and drops the ones
that do not exist. This is load-bearing: TUIs truncate long paths to fit their
width, and a fragment is indistinguishable from a real path by shape. Extraction
is one pass over the capture with no process per line (the bash version once
spawned ~6000); `atelier/tests/pick.rs` covers both against captured fixtures.

Copy is `set-buffer -w`, i.e. OSC 52 through tmux (`set-clipboard on`), so it
reaches the local clipboard over SSH as well. Every action on the placeholder
row is a no-op, as in the other pickers.

## Markdown preview

`<leader>mr` in a markdown buffer runs `atelier preview <file>`, which focuses
that file's Chrome tab or opens one, and prints its URL. Rendering is
client-side (markdown-it, highlight.js, mermaid); the page re-renders on every
save and follows the nvim cursor. Its real purpose is **annotation**: comment on
line ranges in the margin, then "copy all" yields `path` + `L12-L14: comment`
lines to paste into an agent.

**One permanent server, `atelier preview serve`, run as a service** (see "Services")
(`com.github.gauthierdoppler.md-preview`, `127.0.0.1:33440`; `MD_PREVIEW_PORT`
overrides the port for both commands), not one per file. The URL path *is* the
file's absolute path, so the origin never changes, relative images resolve
natively, and a link to another `.md` renders instead of downloading. It used to
be a server per file on a port hashed from the path, bumped on collision and
killed after 10 minutes without a client: Chrome suspends background tabs, which
drops SSE, so the server died under any tab left in the background, and a bumped
port changed the origin — which is what notes were keyed on.

**Everything the page loads is compiled into the binary**: `index.html` and
`app.js` from `atelier/assets/preview/`, and the eight library files under
`atelier/assets/preview/lib/` (markdown-it 14.1.0, markdown-it-anchor 9.2.0,
markdown-it-footnote 4.0.0, markdown-it-task-lists 2.1.1, highlight.js 11.11.1
from `@highlightjs/cdn-assets`, mermaid 11.4.1, DOMPurify 3.2.6, js-yaml 4.1.0;
licences alongside), served from `/__lib/`. A fresh clone builds without bun and
the preview works offline. **Editing the page means rebuilding atelier**: a
change to `index.html` or `app.js` shows up only after `cargo install` (or
`./install.sh`) replaces the binary, which the running server then picks up by
restarting itself (below). There is no Bun server, `package.json` or
`node_modules` any more; bun is still installed, for grove.

**Notes live in `~/.local/share/md-preview/notes/<sha1 of path>.json`**, through
`GET`/`PUT /__notes/<path>`, never in `localStorage`. Moving or renaming a file
loses its notes; that is accepted. The file format is the old Bun server's,
byte for byte (`tests/fixtures/preview/notes-written-by-bun.json` was written by
it), so notes from before the port load unchanged. The page's "copy all" and
cursor-follow logic live in `app.js` and are pinned by `tests/preview.rs`.

**The server restarts itself when its binary changes** (the path it was started
from changes mtime or size, which `cargo install` does — and so does any edit to
the embedded page), via `launchctl
kickstart -k` under launchd and by exiting anywhere else. Exiting and relying on
`KeepAlive` does not work: launchd marks the respawn `pended nondemand spawn =
inefficient` and defers it for minutes, whatever the exit code. systemd's
`Restart=always` has no such deferral, only its 10 s `RestartSec`. `atelier
preview` starts the service itself when `/__meta` does not answer — `launchctl
kickstart` on macOS, `systemctl --user start` elsewhere, falling back on Linux to
a detached `atelier preview serve` when no unit answers or `MD_PREVIEW_PORT` is
not the default the unit serves.

**The page renders untrusted markdown on an origin that can read local files**,
so it is fenced on four sides, each tested over HTTP in `atelier/tests/preview.rs`:

- `DOMPurify` over markdown-it's output (`html: true` stays, so `<details>` and
  sized `<img>` from GitHub READMEs still work) and mermaid in `strict`;
- a CSP with `script-src 'self'` — every script is a file, nothing is inline —
  sent with `nosniff` on every response, so an `.html` or `.svg` served to a page
  cannot run script on the origin either;
- the `Host` header must be `127.0.0.1` or `localhost` with the server's port,
  against DNS rebinding;
- a non-markdown file is only served to a page whose `Referer` is a markdown
  file in the same repo (or the same directory outside a repo), after resolving
  symlinks. Markdown files themselves are served from anywhere: they are what
  you ask to open.

Writes require `content-type: application/json`, which a cross-origin page
cannot send without a preflight this server never answers.

Tab reuse is macOS only: JXA against Google Chrome (`w.tabs.url()` per window,
matched without the fragment); the first run triggers macOS's automation prompt.
If Chrome is not running or the script fails, it falls back to `open -a`. On
other systems the URL goes to `xdg-open` when it is on `PATH`, and is printed
either way.

**`Prefix + m` picks a markdown file of the session** (`atelier preview pick`,
`atelier/src/preview/picker.rs`) and previews it, so reviewing an agent's plan
needs no nvim. Rows are paths relative to `#{session_path}` — the session's own
checkout, so a worktree session lists its worktree — newest mtime first. Inside a
repo they come from `git ls-files -co --exclude-standard`, so gitignored files
are left out; elsewhere from a walk that skips dot-directories and stops at six
levels, so a session rooted at `$HOME` stays usable. `atelier preview list`
prints the rows `tests/markdown_picker.rs` checks. Like `sessions pick`, it
resolves the session itself, since tmux 3.4 does not expand formats in
`display-popup`'s command. The binding replaces tmux's default `m` (mark pane),
which nothing here used.

The cursor sync is a `CursorHold` autocmd, registered once `<leader>mr` has run
in that buffer, that `POST`s the line to `/__cursor/<path>`; the URL comes from
the CLI's stdout, so the port is defined in atelier alone. The page only
scrolls when the line's block is outside the middle of the viewport, so it does
not twitch on every cursor move.

## Theming

Catppuccin across the stack: **Frappe** for ghostty, tmux and neovim; Mocha for
lazygit. `ghostty/config` pins `font-family = JetBrainsMono Nerd Font Mono` —
without it `font-family` is empty, ghostty falls back to a font with no Nerd
Font coverage, and every powerline separator and icon renders as a blank cell.
`have_nerd_font = true` in neovim depends on that pin.
