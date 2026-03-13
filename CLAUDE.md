# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

A macOS dotfiles repo managing configs for: zsh, tmux, git, neovim, ghostty, zed, lazygit, lazydocker, and delta. All configs are symlinked from this repo to their expected locations via `install.sh`.

## Installation & Symlinks

```bash
git clone --recursive git@github.com:GauthierDoppler/dotfiles.git ~/dotfiles
cd ~/dotfiles && ./install.sh
```

The `link()` function in `install.sh` creates symlinks and backs up existing files as `*.bak`. When adding a new config, use the `/add-config` skill.

**Naming convention:**
- `~/.config/*` folders → stored as-is (e.g., `ghostty/`, `lazygit/`)
- `~/` dotfiles → prefixed with `dot_` (e.g., `dot_zshrc` → `~/.zshrc`)
- `~/.config/*` individual files → keep parent structure (e.g., `git/ignore`)

## Neovim Config (submodule)

The `nvim/` directory is a git submodule pointing to a custom fork of kickstart.nvim (`GauthierDoppler/kickstart.nvim`). Changes to neovim config must be committed inside the submodule first, then the submodule ref updated in this repo.

**Structure:** `nvim/init.lua` loads `config.options`, `config.keymaps`, `config.autocmds`, `config.lazy` (in that order). All plugins are managed by lazy.nvim in `lua/config/lazy.lua`. Custom plugins go in `lua/custom/plugins/`, kickstart extras in `lua/kickstart/plugins/`.

**LSP keymaps** are consolidated in a single `LspAttach` autocommand inside the Telescope config section of `lazy.lua`. The lspconfig `LspAttach` handles only document highlight and inlay hints. `grn`/`gra` are Neovim 0.11+ built-in defaults (not explicitly mapped).

**Formatting:** Stylua with 2-space indent, single quotes (see `nvim/.stylua.toml`).

**Smoke test:** `nvim/scripts/test-config.sh` runs headless Neovim validation.

## Key Integrations

- **Tmux ↔ Neovim**: `vim-tmux-navigator` for Ctrl+h/j/k/l pane navigation. Tmux has `focus-events on` for Neovim autoread and gitsigns refresh.
- **Git ↔ Delta**: `dot_gitconfig` includes `delta/themes.gitconfig` for diff rendering. Lazygit also uses delta with custom side-by-side/inline pagers.
- **Ghostty ↔ Tmux**: Extended key sequences for Shift+Enter compatibility.
- **Tmux modes**: Zellij-style modal keybindings (Prefix → p/t/R/m for pane/tab/resize/move).

## Theming

Catppuccin across the stack: Mocha for tmux/lazygit, Frappe for ghostty/neovim. `have_nerd_font = false` in neovim.
