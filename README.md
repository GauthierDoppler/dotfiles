# dotfiles

Personal development environment for macOS — managed with symlinks.

## What's inside

| Config | Path | Description |
|--------|------|-------------|
| **Zsh** | `dot_zshrc` | Oh My Zsh with robbyrussell theme, fnm, bun, Android SDK paths |
| **Tmux** | `dot_tmux.conf` | Zellij-style modal keybindings, Catppuccin Mocha theme, Neovim integration |
| **Git** | `dot_gitconfig` | Worktree helpers (`git wt`), skip/unskip aliases, delta pager |
| **Neovim** | `nvim/` | [kickstart.nvim](https://github.com/GauthierDoppler/kickstart.nvim) (submodule) |
| **Ghostty** | `ghostty/config` | Catppuccin Frappe theme, split navigation keybinds |
| **Zed** | `zed/` | Anthropic default model, One Dark theme, auto-format TypeScript |
| **Lazygit** | `lazygit/config.yml` | Delta side-by-side and inline pagers (toggle with `\|`) |
| **Delta** | `delta/themes.gitconfig` | Custom themes for lazygit (side-by-side + inline) |
| **Git ignore** | `git/ignore` | Global gitignore (`.claude/settings.local.json`) |

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
dot_zshrc          → ~/.zshrc
dot_tmux.conf      → ~/.tmux.conf
dot_gitconfig      → ~/.gitconfig
nvim/              → ~/.config/nvim
ghostty/           → ~/.config/ghostty
zed/               → ~/.config/zed
lazygit/           → ~/.config/lazygit
delta/             → ~/.config/delta
git/ignore         → ~/.config/git/ignore
```

### Machine-specific config

Add local overrides (project paths, work tools) to `~/.zshrc.local` — it's sourced last and not tracked in git.

### SSH helper

`ssh-setup [name]` — generates an ed25519 key, adds to agent, copies pubkey to clipboard. Available after install.

## Dependencies

Auto-installed by `install.sh` via `Brewfile`:

- [Neovim](https://neovim.io/), [tmux](https://github.com/tmux/tmux), [Ghostty](https://ghostty.org/), [Raycast](https://raycast.com/), [Claude](https://claude.com/)
- [fnm](https://github.com/Schniz/fnm), [bun](https://bun.sh/), [Go](https://go.dev/)
- [delta](https://github.com/dandavison/delta), [lazygit](https://github.com/jesseduffield/lazygit), [lazydocker](https://github.com/jesseduffield/lazydocker), [ripgrep](https://github.com/BurntSushi/ripgrep)
- [Oh My Zsh](https://ohmyz.sh/), [pipx](https://pipx.pypa.io/), [terminal-notifier](https://github.com/julienXX/terminal-notifier)

## Tmux cheatsheet

Prefix is `Ctrl+a`. Modes are displayed in the status bar. `Prefix → ?` opens this as a popup.

The status bar answers **where am I**; the menus answer **where can I go**. It reads `bp-api_main ····  1 cc  2 nvim  3 dev`: the worktree you're in, one dot per *other* live session, then the windows of this session.

Listing every session in the bar cost ~95 of 120 columns and drowned the one thing you read continuously — the current window. The dots keep the awareness at 4 columns instead of 95, and a dot turns **yellow** when that session has a window in alert, so you see that a Claude finished elsewhere without carrying the list around. `Prefix → s` resolves a dot into a name.

Session names are shortened — `grove_bp-api_main_f2d1` renders as `bp-api_main`. Window markers: `•` activity, `!` bell, `󰊓` zoomed. The active window is the only saturated block on the bar, in the same blue as the active pane border.

### Sessions

Switching uses `switch-client`, not `attach-session`, so it works from inside a session — no new terminal tab.

| Shortcut | Action |
|----------|--------|
| `Prefix → s` / `Alt+s` | Session menu — numbered popup, grove's `grove_` prefix and hash trimmed, `•` marks a session in alert |
| `Prefix → Tab` | Last session (ping-pong between two worktrees) |
| `Prefix → ( )` | Previous / next session (repeatable) |
| `Prefix → S` | Full `choose-tree` picker |

### Windows

| Shortcut | Action |
|----------|--------|
| `Alt+h` / `Alt+l` | Previous / next window (no prefix) |
| `Alt+1-9` | Jump to window N (no prefix) |
| `Alt+o` | Last window (no prefix) |
| `Prefix → h` / `l` | Previous / next window (repeatable) |
| `Prefix → Space` | Last window |
| `Prefix → w` / `Alt+w` | Menu of every window in every session — jump anywhere in one hop |
| `Prefix → 1-9` | Jump to window N |

Alt bindings need `macos-option-as-alt = left` in the ghostty config (set by default here). Right Option still types `{ } [ ] | \`.

### Look & feel

Menus and popups are themed rather than left to tmux's white-on-black defaults: Catppuccin Mocha ground, rounded borders (`menu-border-lines` / `popup-border-lines`), and both menu scripts pad their columns to a measured width so entries don't stair-step. The inactive pane is dimmed slightly (`window-style` vs `window-active-style`) and the active pane border is blue, so the focused pane reads without hunting. Window activity and bell show as a `•` / `!` marker only — tmux's default `reverse` style painted a grey block over half the bar.

### Modes

| Shortcut | Action |
|----------|--------|
| `Prefix → p` | **Pane mode** — `d` split down, `r` split right, `x` close, `z` zoom, `hjkl` navigate |
| `Prefix → t` | **Tab mode** — `n` new, `x` close, `,` rename, `hl` prev/next, `1-9` jump |
| `Prefix → R` | **Resize mode** (sticky) — `hjkl` resize 2px, `HJKL` resize 5px, `Esc` exit |
| `Prefix → m` | **Move mode** (sticky) — `hjkl` swap panes directionally, `Esc` exit |
| `Ctrl+hjkl` | Navigate panes (no prefix, Neovim-aware) |
| `Prefix → [` | Copy mode (vi keys: `v` select, `y` copy, `/` search) |

### Plugins

Managed by [TPM](https://github.com/tmux-plugins/tpm), bootstrapped by `install.sh`. `Prefix → I` installs, `Prefix → U` updates.

- **tmux-resurrect** + **tmux-continuum** — auto-saves every 15 min and restores sessions, windows, layouts and working dirs when the tmux server starts, so worktree sessions survive a reboot or a ghostty crash.

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
