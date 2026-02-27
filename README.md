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
git clone --recursive git@github.com:GauthierDoppler/dotfiles.git ~/dotfiles
cd ~/dotfiles
./install.sh
```

The install script symlinks everything to the right place. Existing files are backed up as `*.bak`.

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

## Dependencies

- [Oh My Zsh](https://ohmyz.sh/)
- [fnm](https://github.com/Schniz/fnm) — fast Node version manager
- [bun](https://bun.sh/)
- [Neovim](https://neovim.io/)
- [tmux](https://github.com/tmux/tmux)
- [Ghostty](https://ghostty.org/)
- [delta](https://github.com/dandavison/delta) — git diff pager
- [lazygit](https://github.com/jesseduffield/lazygit)

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
