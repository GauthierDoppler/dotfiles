# 08: Task placements: split, popup and detach

**What to build:** The remaining placements from the `# tmux:` header, with optional split size. The bash task picker and runner are deleted and the tmux-tasks skill is updated.

**Blocked by:** 07 (Task picker and window tasks)

**Status:** done

- [x] `split-down` and `split-right` with and without a percentage, default 30%
- [x] `popup` reuses the picker's popup instead of opening a nested one
- [x] `detach` runs unselected and marks its own window, not the active one
- [x] Only window and detach placements are marked and bell
- [x] Bash scripts removed, skill and `AGENTS.md` updated

## Comments

- **Every placement now runs under `atelier tasks exec`.** Splits use `split-window -v|-h -l <size>` on the session's active pane; `split` alone still means `split-down`, as in the bash. An unknown placement falls back to `window`, as before.
- **`detach` is re-run with `respawn-pane`, not `respawn-window`.** `respawn-window` has no `-d` and makes the window current, so re-running a detached task pulled it to the front (the bash had the same bug). Covered by `rerunning_a_detached_task_reuses_its_window_without_selecting_it`.
- **A popup picked in the picker runs in the picker's own process.** `atelier tasks run <popup task>` outside the picker opens its own `display-popup`, spawned and not awaited, because `display-popup` from the command line only returns once the popup closes. That path cannot report errors (no client attached, say): it silently does nothing.
- **The popup test** runs `atelier tasks pick` inside a real `display-popup` on a client attached through `script`, with a stub `fzf` on `PATH` that picks the first row. Forcing the nested-popup path makes it fail, so it does catch the regression.
- **No fallback without atelier any more.** `Prefix + e` calls `atelier tasks pick` only, like `Prefix + s`/`Space`. Without atelier, tasks are still plain scripts that run by hand.
- **The `terminal-notifier` banner for `detach` tasks is gone** with the bash runner: tasks ring only (user story 23; the spec keeps notifications for Claude's `waiting`).
- **`shell_quote` is one helper**, `src/shell.rs` (`shell::quote`), used by `sessions.rs`, `pick.rs`, `tasks/picker.rs` and `preview/picker.rs` (which arrived with a fourth copy). The "atelier exe + `--socket`" prefix they build is still duplicated (`pick` spells it `-S`); left alone to keep this branch's footprint in other features small.
- **Not verified on macOS:** the popup and split tests rely on `script` and tmux's default 80x24 detached size; CI covers macOS.
