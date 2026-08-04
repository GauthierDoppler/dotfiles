# dotfiles

Personal development environment for macOS — managed with symlinks.

## What's inside

| Config | Path | Description |
|--------|------|-------------|
| **Zsh** | `dot_zshrc` | Oh My Zsh with robbyrussell theme, fnm, bun, keg-only brew paths, `project` jump aliases |
| **Zsh (login env)** | `dot_zprofile` | Locale, `JAVA_HOME`, Android SDK — each block inert when the tool is absent |
| **Tmux** | `dot_tmux.conf` | Pane/tab modes, git-aware status bar, session + task + URL pickers, Catppuccin Frappe |
| **Git** | `dot_gitconfig` | Worktree helpers (`git wt`), skip/unskip aliases, delta pager |
| **Neovim** | `nvim/` | originally forked from [kickstart.nvim](https://github.com/nvim-lua/kickstart.nvim), since diverged |
| **Ghostty** | `ghostty/config` | Catppuccin Frappe theme, split navigation keybinds |
| **Zed** | `zed/` | Anthropic default model, One Dark theme, auto-format TypeScript |
| **Lazygit** | `lazygit/config.yml` | Delta side-by-side and inline pagers (toggle with `\|`) |
| **Delta** | `delta/themes.gitconfig` | Custom themes for lazygit (side-by-side + inline) |
| **Git ignore** | `git/ignore` | Global gitignore (`.claude/settings.local.json`) |
| **Keyboard** | `keyboard/` | AZERTY layout with an unshifted number row — the tmux `Prefix → 1-9` bindings depend on it |

## Install

```bash
xcode-select --install
git clone https://github.com/GauthierDoppler/dotfiles.git ~/dotfiles
cd ~/dotfiles && ./install.sh
```

First run generates an SSH key and exits. Add the key to GitHub, then run `./install.sh` again for the full setup (Homebrew, CLI tools, desktop apps, Oh My Zsh, symlinks, Node, Claude Code CLI).

All dependencies are auto-installed via `Brewfile`. Existing files are backed up as `*.bak`.

### Symlink map

```
dot_tmux.conf      → ~/.tmux.conf
nvim/              → ~/.config/nvim
ghostty/           → ~/.config/ghostty
zed/               → ~/.config/zed
lazygit/           → ~/.config/lazygit
delta/             → ~/.config/delta
git/ignore         → ~/.config/git/ignore
lazydocker/        → ~/.config/lazydocker
dot_claude/CLAUDE.md            → ~/.claude/CLAUDE.md
dot_claude/hooks                → ~/.claude/hooks
dot_claude/statusline-custom.sh → ~/.claude/statusline-custom.sh

keyboard/FR-AZERTY-num.bundle   → ~/Library/Keyboard Layouts/  (copied, not linked)
```

`~/.zshrc`, `~/.zprofile`, `~/.gitconfig` and `~/.claude/settings.json` are
deliberately **not** symlinked — see below.

### Machine-specific config

Every tracked config is portable. **`~/.zshrc`, `~/.zprofile` and `~/.gitconfig`
are stubs, not symlinks**: `install.sh` writes a small real file that loads the
tracked config, and everything machine-local accumulates below that line.

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

This exists because third-party installers append to those files directly.
Through a symlink those appends land in tracked files, which is how a
`/Users/<name>` socket path once got staged here. `git config --global` writes
to the stub too, which is now the correct outcome — and for git, later values
win, so anything below the include overrides the shared config.

| Shared (tracked)           | Local (untracked)                 |
| -------------------------- | --------------------------------- |
| `dot_zshrc`                | `~/.zshrc`, below the load line    |
| `dot_zprofile`             | `~/.zprofile`, below the load line |
| `dot_gitconfig`            | `~/.gitconfig`, below the include  |
| `dot_claude/settings.json` | `dot_claude/settings.local.json`  |

`local-diff` lists what has accumulated locally, so you can decide whether to
promote it into the tracked config or leave it machine-specific. Some of what it
reports is permanently local — project registrations below are the main case, and
they stay flagged on every run rather than being filtered out, because a diff
tool that silently drops lines stops being trustworthy for the ones that matter.

#### Project setups

`project <name> <path> [tab name]` defines a shell function that enters a
project in its default state: `cd`, rename the ghostty tab, and `grove attach`
on the main worktree's branch — creating that session if it does not exist, or
switching to it if you are already in tmux. So `biogroup` gets you from anywhere
to the right project on its main session.

The function is tracked in `dot_zshrc`; the registrations are not, since they
carry absolute paths. Add them to `~/.zshrc` **below the load line**:

```sh
project biogroup ~/Developer/theodo/bp-api "Biogroup"
project grove    ~/Developer/perso/grove-ai
```

Three fields, and nothing per-project anywhere else. The tab name defaults to
the project name. You get tab-completion on the name for free — it is an
ordinary shell function.

A registration whose path does not exist is skipped silently, so one block can
be carried to a machine that has not cloned everything. A name that already
resolves to a command is skipped with a warning, rather than shadowing it. With
no `grove` on `PATH`, or a path that is not a git repo, the `cd` still happens
and nothing else does.

Claude Code is the awkward one: it has no user-scope local settings file, and it
rewrites `~/.claude/settings.json` in place (reordering keys, absolutising
paths). So that file is generated rather than linked. `claude-settings-sync`
merges the tracked base with the local override, and before each regeneration it
captures whatever the app changed into the local override — so app-side edits are
never lost and the shared base still propagates. `--dry-run` previews it.

### Helpers

Symlinked into `~/.local/bin` by `install.sh`:

- `ssh-setup [name]` — generates an ed25519 key, adds to agent, copies pubkey to clipboard
- `local-diff` — shows what this machine's local config adds beyond the tracked dotfiles
- `claude-settings-sync` — regenerates `~/.claude/settings.json` from base + local override
- `tmux-sessions` — session picker behind `Prefix + Space`
- `tmux-tasks` / `tmux-task-run` — task picker behind `Prefix + e`, and its runner
- `tmux-pick` — URL/path picker behind `Prefix + u`
- `tmux-status-left` — renders project · root/wt · branch in the status bar

## Dependencies

Auto-installed by `install.sh` via `Brewfile`:

- [Neovim](https://neovim.io/), [tmux](https://github.com/tmux/tmux), [Ghostty](https://ghostty.org/), [Raycast](https://raycast.com/), [Claude](https://claude.com/)
- [fnm](https://github.com/Schniz/fnm), [bun](https://bun.sh/), [Go](https://go.dev/)
- [delta](https://github.com/dandavison/delta), [lazygit](https://github.com/jesseduffield/lazygit), [lazydocker](https://github.com/jesseduffield/lazydocker), [ripgrep](https://github.com/BurntSushi/ripgrep)
- [Oh My Zsh](https://ohmyz.sh/), [pipx](https://pipx.pypa.io/), [terminal-notifier](https://github.com/julienXX/terminal-notifier)
- [stylua](https://github.com/JohnnyMorganz/StyLua) — CI checks `nvim/` formatting, so it must be runnable locally

## Tmux cheatsheet

Prefix is `Ctrl+a`. Modes are displayed in the status bar.

| Shortcut | Action |
|----------|--------|
| `Prefix → 1-9` | Jump to tab N (N > count → last tab) |
| `Prefix → p` | **Pane mode** — `d` split down, `r` split right, `x` close, `z` zoom, `hjkl` navigate |
| `Prefix → t` | **Tab mode** — `n` new, `x` close, `,` rename, `hl` prev/next, `Tab` last used |
| `Prefix → Space` | Session picker (`s` does the same) |
| `Prefix → e` | Task picker — scripts from `<project>/.tmux/` |
| `Prefix → u` | Pick a URL or path off the pane — `Enter` opens, `Ctrl-y` copies |
| `Prefix → v` | Copy mode, then select with the mouse (`[` is an alias) |
| `Prefix → L` | Next layout — the keyboard way to rebalance a split |
| `Ctrl+hjkl` | Navigate panes (no prefix, Neovim-aware; also inside copy mode) |

Resize and move modes were removed — unused, and dragging a pane border resizes.

### Keyboard layout (one manual step)

`Prefix → 1-9` assumes the number row types digits **without** Shift.
`install.sh` copies `keyboard/FR-AZERTY-num.bundle` (AZERTY with a
QWERTY-order unshifted number row, authored in Ukelele) into
`~/Library/Keyboard Layouts/` — copied, not symlinked, because macOS's
input-source daemon does not reliably follow a symlink there. Re-running
`install.sh` refreshes it. But macOS will not let a script pick it for you —
writing `com.apple.HIToolbox` with `defaults` is cached by the input-source
daemon and only takes after a logout.

So after the first install, once, by hand:

1. **System Settings → Keyboard → Text Input → Input Sources → Edit**
2. `+` → **French** → **Français AZERTY Numérique** → Add
3. Select it (and remove the stock AZERTY if you want it gone)

The layout's menu-bar icon is not tracked (322 KB of `.icns` for a glyph), so it
shows a generic icon. Nothing else changes.

**Copying:** `Prefix + u` for a token (URL, file path), `Prefix + v` for a
region. Plain dragging is unreliable — tmux only selects when the pane's
application has not grabbed the mouse, which Neovim always does and Claude Code
does intermittently. Ghostty's `Shift`+drag also works but selects by screen
column, so it ignores split boundaries.

Status bar shows **project · root/wt · branch**, resolved with git rather than
from the session name, plus per-window task markers: `●` running, `✓` ok,
`✗` failed.

## Git aliases

| Alias | Description |
|-------|-------------|
| `git wt <branch> [base]` | Create a worktree under `worktrees/<branch>` |
| `git wtl` | List worktrees |
| `git wtc <branch>` | `cd` into a worktree |
| `git wtr <branch>` | Remove a worktree |
| `git wtp` | Prune stale worktrees |
| `git skip <file>` | Hide a file from `git status` (skip-worktree or exclude) |
| `git unskip <file>` | Undo `git skip` |
| `git sync` | Fetch + rebase on `origin/main` |
| `git pullsafe` | Stash, pull, stash pop |
