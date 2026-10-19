# AGENTS.md

This file provides guidance to AI coding agents (Claude Code, Codex) when working
with code in this repository. `CLAUDE.md` is a symlink to this file. It records
rules and the rationale that still decides things. The tool behind the tmux
pickers, bar and installers is atelier, in its own repo; its `AGENTS.md` holds
the implementation rules and the tests that enforce them.

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
preview) declared in `services.toml`.
[atelier](https://github.com/GauthierDoppler/atelier) (private) is the Rust
binary that owns the behaviour (status bar, pickers, task runner, preview
server, setup, services, doctor); configs stay plain files linked from the repo.

## Installation & Symlinks

```bash
xcode-select --install
git clone https://github.com/GauthierDoppler/dotfiles.git ~/dotfiles
cd ~/dotfiles && ./install.sh    # first run: SSH key setup, second run: full install
```

`install.sh` is only the bootstrap (Homebrew, atelier, the keyboard layout,
Node, then `atelier setup`, `claude-settings-sync` and `service install`); it
stops if the atelier build fails. **atelier is cloned once to
`~/Developer/perso/atelier`** (`$ATELIER_DIR` overrides; private, so over SSH)
and built from that checkout as it stands: `install.sh` never pulls it, so a
change there is installed by re-running `install.sh` or `cargo install --locked
--path . --root ~/.local` in it.

`atelier setup [--profile desktop|remote] [--dry-run]` links and stubs from
**`links.toml`**, the one list of what goes where (format in atelier's
`AGENTS.md`); use the `/add-config` skill to add an entry. It backs up as
`*.bak`, `*.bak.1`, …, is idempotent, refuses a folder without `links.toml`,
and prunes only dangling links into the repo. Profile `remote` (default off
macOS) skips `desktop_only` links, casks and the keyboard layout.

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
A stub is written only when its load line is missing.
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
decision: promote it, or leave it local. It walks the stubs in `links.toml`, so
a new stub needs no second list.

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
captured as local drift and can never propagate again. Both the override and
the snapshot are gitignored, and `[claude_settings]` in `links.toml` names all
three. Preview with `--dry-run`.

## atelier

Lives in [GauthierDoppler/atelier](https://github.com/GauthierDoppler/atelier);
its `AGENTS.md` holds the code rules, the mechanics of every feature below and
the tests that enforce them. What this repo owes it:

- **`links.toml` and `services.toml`** describe this checkout; atelier knows no
  path in it. The markdown preview service keeps the label
  `com.github.gauthierdoppler.md-preview` on `127.0.0.1:33440`.
- **Everything calls `~/.local/bin/atelier` by absolute path** (hooks run with a
  thin `PATH`) and must still work when it is absent: `dot_tmux.conf` guards on
  `-x`, the bar falls back to `#()` and the session name.
- **The bar formats and the `after-select-window` hook in `dot_tmux.conf` are
  mirrored** in atelier's `tests/fixtures/dotfiles/dot_tmux.conf`; change them
  in both repos together.
- Needs tmux ≥ 3.4 and fzf ≥ 0.59; `atelier doctor` checks the machine.

## Tmux

**There is no session persistence — no tmux-resurrect, no tmux-continuum, no
tpm — and that is deliberate.** `grove` (see the `grove` skill) is the session
factory: a session is derived from a worktree plus `.grove/config.yaml`, so
recreating it reconstructs what the project *is*, where a save-file restores
whatever was open at the last checkpoint. Do not add a persistence plugin.

**`terminal-features`, not `terminal-overrides`, and never with a bare `set -a`**:
appending to an array option grows it on every `Prefix + r`. `set -gu` first
(restoring the default, which carries the stock clipboard and focus entries),
then `set -as` (`atelier doctor` fails a grown list).

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

## Pickers, tasks and the Claude Code marker

All atelier, opened from `dot_tmux.conf`; behaviour and mechanics are in
atelier's `AGENTS.md`.

| key | runs | what |
| --- | ---- | ---- |
| `Prefix + Space` / `s` | `atelier sessions pick` | sessions of this project, `Tab` for all, `h`/`l` cycle the previewed session's window |
| `Prefix + e` | `atelier tasks pick` | the project's `.tmux/` tasks (the `tmux-tasks` skill covers writing them) |
| `Prefix + u` | `atelier pick popup` | URLs and paths in the pane: `Enter` open, `Ctrl-y` copy (OSC 52), `Ctrl-o` OS opener, `Ctrl-v` preview markdown |
| `Prefix + m` | `atelier preview pick` | a markdown file of the session's checkout, in the preview |

Every picker is the same fzf: `j`/`k` move, `i` searches, `esc` leaves search
then closes. `.tmux/` is ignored globally (`git/ignore`).

**`@task_status`** (`●`/`✓`/`✗`) and **`@claude_status`** (yellow `✻` waiting,
green `✻` done) share one slot in the window format. `atelier hook claude` is
one command for `Notification`, `Stop` and `UserPromptSubmit` in
`dot_claude/settings.json`, and **always ends in `|| true`**: Claude Code
treats exit 2 as blocking (on `UserPromptSubmit` it discards the prompt), and 2
is clap's exit on an unknown subcommand, so a stale atelier would eat every
prompt. `after-select-window` clears the marker: going to look dismisses it.

**Notifications policy: the marker is the only signal from Claude Code** — no
bell, `preferredNotifChannel: notifications_disabled`. A beep or banner that
cannot be attributed to a window stops being trusted. Tasks still ring.

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
alternatives are in atelier's `docs/design-notes.md`.

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
agents on macOS and systemd user units on Linux, rewriting and reloading only
what changed, so re-running `install.sh` does not bounce the proxy. `run` is
relative to `$HOME` (launchd expands neither `~` nor `$HOME`). The launchd and
systemd gotchas are in atelier's `AGENTS.md`; the two that need a hand:
`~/Library/LaunchAgents` can end up root-owned (Pulse Secure did it), and
`install` then fails with the `sudo chown` fix; without linger, systemd user
services die at SSH logout, so `install` enables it.

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

atelier renders the left block (project + `root`/`wt`) and the right block (key
table, counts, battery, date, clock) and pushes them as `@bar_left` /
`@bar_right`; `dot_tmux.conf` falls back to `#()` and then the session name.
The window list in between is plain tmux format. Layout and colour rules for
both sides are in atelier's `AGENTS.md`.

- **Truncate inside atelier, never via `status-left-length`**: tmux truncates the
  *expanded* string and will cut a `#[fg=...]` in half.
- **Escape a comma in `#[...]` as `#,` inside a `#{?...}`**, or it ends the branch
  and the rest of the style is silently dropped: `#[bg=#a6d189#,fg=#303446]`.
- **A window has one width active or not**: the inactive format replaces the
  capsules with spaces, and the marker slot is +3 on both.
- **Colour: the window list is greyscale** — blue `#8caaee` is focus,
  yellow/green/red are state; the marker is never drawn on the blue. Catppuccin
  **Frappe**; bar `#414559` on terminal `#303446`.
- **The `` `` caps are U+E0B6 / U+E0B4** (Nerd Font, private-use area):
  some editors silently drop them on write — check with `grep -c` after touching
  a file that holds them.

## Copying out of a pane

- **`Prefix + u`** (`atelier pick popup`) — fuzzy-pick a URL or file path; see
  the picker table above.
- **`Prefix + v`** — copy-mode, then drag, for a **region**. tmux's default
  `MouseDrag1Pane` only selects when the app has not grabbed the mouse (Neovim
  always has); copy-mode owns the mouse and respects pane borders.

## Markdown preview

`<leader>mr` runs `atelier preview <file>` (focus or open its browser tab — tab
reuse is JXA, macOS only); the page re-renders on save and follows the nvim
cursor. Its purpose is **annotation**: "copy all" yields `path` + `L12-L14:
comment` lines for an agent. One permanent server, `atelier preview serve`, runs
as a service on `127.0.0.1:33440`. The page is compiled into atelier, so editing
it means rebuilding atelier. The server has no authentication: keep it off a
machine other users can log into.

## Theming

Catppuccin across the stack: **Frappe** for ghostty, tmux and neovim; Mocha for
lazygit. `ghostty/config` pins `font-family = JetBrainsMono Nerd Font Mono` —
without it ghostty falls back to a font with no Nerd Font coverage and every
powerline separator and icon renders blank. `have_nerd_font = true` in neovim
depends on that pin.
