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
lazydocker/        → ~/.config/lazydocker
dot_claude/CLAUDE.md            → ~/.claude/CLAUDE.md
dot_claude/hooks                → ~/.claude/hooks
dot_claude/statusline-custom.sh → ~/.claude/statusline-custom.sh
```

`~/.claude/settings.json` is deliberately **not** symlinked — see below.

### Machine-specific config

Every tracked config is portable. Anything specific to one machine (project
paths, work tools, credential helpers) goes in an untracked local file:

| Shared (tracked)           | Local (untracked)                |
| -------------------------- | -------------------------------- |
| `dot_zshrc`                | `~/.zshrc.local` (sourced last)  |
| `dot_gitconfig`            | `~/.gitconfig.local` (included)  |
| `dot_claude/settings.json` | `dot_claude/settings.local.json` |

`local-diff` lists what has accumulated locally, so you can decide whether to
promote it into the tracked config or leave it machine-specific.

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

## Dependencies

Auto-installed by `install.sh` via `Brewfile`:

- [Neovim](https://neovim.io/), [tmux](https://github.com/tmux/tmux), [Ghostty](https://ghostty.org/), [Raycast](https://raycast.com/), [Claude](https://claude.com/)
- [fnm](https://github.com/Schniz/fnm), [bun](https://bun.sh/), [Go](https://go.dev/)
- [delta](https://github.com/dandavison/delta), [lazygit](https://github.com/jesseduffield/lazygit), [lazydocker](https://github.com/jesseduffield/lazydocker), [ripgrep](https://github.com/BurntSushi/ripgrep)
- [Oh My Zsh](https://ohmyz.sh/), [pipx](https://pipx.pypa.io/), [terminal-notifier](https://github.com/julienXX/terminal-notifier)

## Tmux cheatsheet

Prefix is `Ctrl+a`. Modes are displayed in the status bar.

| Shortcut | Action |
|----------|--------|
| `Prefix → p` | **Pane mode** — `d` split down, `r` split right, `x` close, `z` zoom, `hjkl` navigate |
| `Prefix → t` | **Tab mode** — `n` new, `x` close, `,` rename, `hl` prev/next, `1-9` jump |
| `Prefix → R` | **Resize mode** (sticky) — `hjkl` resize 2px, `HJKL` resize 5px, `Esc` exit |
| `Prefix → m` | **Move mode** (sticky) — `hjkl` swap panes, `Esc` exit |
| `Ctrl+hjkl` | Navigate panes (no prefix, Neovim-aware) |
| `Prefix → [` | Copy mode (vi keys: `v` select, `y` copy, `/` search) |

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
