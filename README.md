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
