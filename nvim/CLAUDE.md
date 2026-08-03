# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

A modular Neovim config based on a custom fork of kickstart.nvim, managed as a git submodule of a dotfiles repo. Uses lazy.nvim for plugin management.

## Validation Commands

```bash
# Smoke test (headless, isolated XDG environment)
scripts/test-config.sh

# Quick startup check
nvim --headless -u init.lua "+qa"
```

## Formatting

Lua files are formatted with Stylua. CI runs `stylua --check` on PRs.

```bash
stylua .
stylua --check .
```

Config: `.stylua.toml` — 2-space indent, 160 col width, single quotes, Unix line endings.

## Architecture

Read `ARCHITECTURE.md` and `AGENTS.md` first — they cover the module layout and edit rules in detail.

**Key points:**
- `init.lua` loads: `config.options` → `config.keymaps` → `config.autocmds` → `config.lazy`
- `lua/config/lazy.lua` is the main plugin spec file (~750 lines, includes kickstart tutorial comments)
- Kickstart extras: `require 'kickstart.plugins.<name>'` in lazy.lua
- Custom plugins: auto-imported via `{ import = 'custom.plugins' }` — add new files in `lua/custom/plugins/`
- `lazy-lock.json` is tracked in git (unlike upstream kickstart) for reproducible installs; the test script restores it after runs

## Edit Rules (from AGENTS.md)

- Keep responsibilities separated: options, keymaps, autocmds, and plugin setup in their respective files.
- Put personal plugin additions in `lua/custom/plugins/*.lua`, grouped by feature (navigation, git, ui, docker).
- Keep plugin-related keymaps close to their plugin config.
- Don't re-expand the modular layout back into a giant `init.lua`.
- All LSP navigation keymaps (`grd`, `grr`, `gri`, `grt`, `grD`, `gO`, `gW`) live in the snacks-picker `LspAttach` autocommand (`lua/custom/plugins/picker.lua`). The lspconfig `LspAttach` (`lua/custom/plugins/lsp.lua`) handles only document highlight and inlay hints. `grn`/`gra` are Neovim 0.11+ built-in defaults.

## LSP Servers Configured

ts_ls, eslint, gopls, pyright, ruff, ruby_lsp, lua_ls — all via mason auto-install.

## This Is a Submodule

This repo is a submodule of `~/dotfiles`. Changes here must be committed inside the submodule first, then the submodule ref updated in the parent repo.
