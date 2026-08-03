# Neovim Config Architecture

This repository is a Kickstart-based Neovim config refactored into modules.

## Entry Point

- `init.lua`
  - Sets global leaders and `vim.g.have_nerd_font`.
  - Loads the main modules in this order:
    1. `lua/config/options.lua`
    2. `lua/config/keymaps.lua`
    3. `lua/config/autocmds.lua`
    4. `lua/config/lazy.lua`

## Core Modules (`lua/config/`)

- `lua/config/options.lua`
  - Editor options (`vim.o`/`vim.opt`): numbers, clipboard, search, split behavior, listchars, etc.

- `lua/config/keymaps.lua`
  - Global non-plugin keymaps.
  - Diagnostic UI config and related keymap(s).
  - Window, buffer, and file-path convenience mappings.

- `lua/config/autocmds.lua`
  - Global autocmds:
    - yank highlight
    - fold setup for Go/TS/Ruby
    - auto-open Neo-tree when starting on a directory

- `lua/config/lazy.lua`
  - `lazy.nvim` bootstrap.
  - Main plugin spec list and plugin configuration.
  - Imports additional custom plugin specs via:
    - `{ import = 'custom.plugins' }`

## Plugin Layers

- `lua/config/lazy.lua`
  - Main/default plugin configuration (Kickstart-style base + your edits).

- `lua/kickstart/plugins/*.lua`
  - Optional Kickstart plugin modules imported from `lua/config/lazy.lua`.
  - Examples currently used: `autopairs`, `neo-tree`, `gitsigns`, `indent_line`.

- `lua/custom/plugins/*.lua`
  - Your local feature-specific plugin specs.
  - Current split:
    - `lua/custom/plugins/navigation.lua`
    - `lua/custom/plugins/git.lua`
    - `lua/custom/plugins/ui.lua`
    - `lua/custom/plugins/init.lua` (placeholder)

## Non-Lua Repo Structure

- `lazy-lock.json`
  - Plugin lockfile managed by `lazy.nvim`.

- `scripts/test-config.sh`
  - Headless smoke test runner with isolated XDG directories.

- `tests/smoke.lua`
  - Smoke checks executed by the script above.

- `doc/`
  - Vim help docs from Kickstart.

## Where To Edit (Quick Guide)

- Add/edit editor options: `lua/config/options.lua`
- Add/edit global keymaps: `lua/config/keymaps.lua`
- Add/edit autocmds/folding behavior: `lua/config/autocmds.lua`
- Add/edit plugin setup:
  - Base plugin specs: `lua/config/lazy.lua`
  - Personal plugin groups: `lua/custom/plugins/*.lua`
- Enable/disable Kickstart optional plugin modules:
  - `require 'kickstart.plugins.<name>'` entries in `lua/config/lazy.lua`

## Validation Workflow

- Preferred local check:
  - `scripts/test-config.sh`
- Quick load check:
  - `nvim --headless -u init.lua "+qa"`

