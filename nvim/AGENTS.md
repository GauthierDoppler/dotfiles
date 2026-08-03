# AGENTS.md

Guidance for AI coding agents working in this Neovim config repository.

## Goal

Keep the config modular and predictable. Prefer small targeted edits over broad rewrites.

## First Files To Read

1. `ARCHITECTURE.md`
2. `init.lua`
3. `lua/config/*.lua`

## Edit Rules

- Do not re-expand the modular layout back into a giant `init.lua`.
- Keep responsibilities separated:
  - options in `lua/config/options.lua`
  - keymaps in `lua/config/keymaps.lua`
  - autocmds in `lua/config/autocmds.lua`
  - plugin setup in `lua/config/lazy.lua`
- Put personal plugin additions in `lua/custom/plugins/*.lua` when possible.
- Keep plugin-related keymaps close to plugin config unless they are truly global.
- Avoid changing behavior unrelated to the request.

## Plugin Changes

- Add base plugins in `lua/config/lazy.lua`.
- Add personal grouped plugins in:
  - `lua/custom/plugins/navigation.lua`
  - `lua/custom/plugins/git.lua`
  - `lua/custom/plugins/ui.lua`
- Preserve `{ import = 'custom.plugins' }` in `lua/config/lazy.lua`.
- Keep `lazy-lock.json` stable unless dependency updates are requested.

## Folding Notes

- Folding is configured in `lua/config/autocmds.lua`.
- Current setup uses Tree-sitter fold expression for:
  - `go`
  - `typescript`
  - `typescriptreact`
  - `javascript`
  - `javascriptreact`
  - `ruby`

## Validation

- Run smoke test after non-trivial changes:
  - `scripts/test-config.sh`
- Quick startup check:
  - `nvim --headless -u init.lua "+qa"`

## Commit Guidance

- Use clear, scoped commit messages:
  - `refactor: ...`
  - `feat: ...`
  - `fix: ...`
  - `docs: ...`
- Do not include unrelated generated artifacts.

