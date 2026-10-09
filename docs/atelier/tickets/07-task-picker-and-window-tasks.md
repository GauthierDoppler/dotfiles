# 07: Task picker and window tasks

**What to build:** `Prefix + e` lists tasks from the session's `.tmux/` folder through atelier and runs a `window` task end to end: project root as cwd, root and name in the environment, window reused by name, running/ok/fail marker, bell on completion when not watched.

**Blocked by:** 02 (Atelier tracer bullet: the bar's project name comes from atelier)

**Status:** done

- [x] Discovery: executable files at depth 1 and 2 only, a non-executable helper never listed
- [x] Headers read from the first 20 lines; missing headers fall back to defaults
- [x] Most recently run first, groups cycled with Tab, placeholder row when empty
- [x] Root resolved from the session's path, identical from a pane three directories deep
- [x] Re-running reuses the window and resets the marker to running first
- [x] All of the above tested through the tmux harness with fixture task folders

## Comments

- **Commands live under `atelier tasks`** (`pick`, `run <name>`, and the fzf callbacks `list`, `advance`, `prompt`, `header`), plus a hidden `tasks exec` that tmux starts as a window task's command. That is the in-window runner the brief called `atelier run`; it sits under `tasks` to keep the feature to one module and one `features!` line.
- **Non-window placements still go through the bash until 08.** `split-*` and `detach` go to `~/.local/bin/tmux-tasks --run`, and `popup` execs `~/.local/bin/tmux-task-run` inside the picker's popup, because tmux allows only one popup per client. `Placement` in `src/tasks/runner.rs` is where 08 adds them. I checked by hand with a real attached client (`script` + fzf 0.65) that `window`, `split-right`, `detach` and `popup` all run from the new picker, and that `Prefix + e` falls back to `tmux-tasks` when `~/.local/bin/atelier` is missing.
- **Window tasks ring a bell and send no `terminal-notifier` banner.** User story 23 asks for a bell only, and the spec limits desktop notifications to Claude's `waiting` state. The bash `detach` path still sends a banner until 08.
- **The "watched" check ignores control-mode clients**, as the spec says. Tests cover three cases: nobody watching (rings), a control-mode client only (rings), and a real client attached through `script` (silent).
- **The MRU and group state moved.** They are now `$TMPDIR/atelier-tasks.<fnv64 of root>.{recent,group}`. The group is per project rather than global, and existing bash MRU history is not migrated.
- **Rows have no trailing padding** when a task has no description. Otherwise they match the bash output.
- **Not verified on macOS:** `wait_for_key` (`stty -icanon -echo min 1`) and the `script` invocation in the test harness (`script -q /dev/null sh -c …`). CI covers macOS.
- **Follow-up:** `shell_quote` now exists three times (`sessions.rs`, `pick.rs`, `tasks/picker.rs`) because these tickets were built in parallel. It is worth one shared helper.
