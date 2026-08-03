---
name: tmux-tasks
description: >-
  Create or edit the per-project build/run/debug scripts that back the tmux task picker
  (`Prefix + e`), stored in `<project>/.tmux/`. Use this WHENEVER the user wants a repeatable
  project command wired up so they can launch it from anywhere in their tmux session — "add a
  task for X", "set up the build/run loop", "make a shortcut to run the app", "add this to my
  tmux tasks", "wire up gradle/xcodebuild/adb/simctl/docker/dev-server", or when a project has
  commands you keep re-typing by hand. Also use when a task misbehaves: wrong window, wrong
  directory, duplicate windows, or a task not appearing in the picker.
---

# tmux-tasks

`Prefix + e` opens a fuzzy picker over the current project's task scripts. It is where build,
run and debug loops live — deliberately **not** in Neovim, so they can be driven from any
window of the session.

Implementation: `~/dotfiles/scripts/tmux-tasks`. Read it if behaviour here is unclear.

## Where tasks live

`<project-root>/.tmux/`, untracked. `.tmux/` is in the user's global gitignore
(`~/dotfiles/git/ignore`), so tasks stay out of version control in **any** repo — including
client repos we must not modify — without touching that project's `.gitignore`.

```
<project>/.tmux/
├── lib/
│   └── common.sh          # NOT executable -> not a task, just a helper
├── android/
│   ├── build
│   └── logcat
├── ios/
│   └── simulator
└── doctor                 # depth 1 -> ungrouped
```

**A task is any executable regular file at depth 1 or 2 under `.tmux/`.** The executable bit is
the only filter. This is why helpers live as non-executable files and need no naming convention
or ignore list.

- Subfolders become groups; `Tab` in the picker cycles `all → android → ios → …`.
- Depth 3+ is invisible. Do not nest deeper.
- Display name is the path relative to `.tmux/`, e.g. `android/build`.
- **Names must not contain spaces** — the picker splits on whitespace.

## Writing a task

```bash
#!/usr/bin/env bash
# task: assemble the debug APK and install it on the connected device
# tmux: window
set -euo pipefail

./gradlew :app:installDebug
```

Two optional header keys, read from the **first 20 lines only**:

| key | meaning |
| --- | --- |
| `# task:` | one-line description shown in the picker |
| `# tmux:` | placement — `window` (default) \| `split-down` \| `split-right` \| `popup` \| `detach` |

Both splits take an optional size after the placement, defaulting to 30%:

```bash
# tmux: split-right        # 30% of the width
# tmux: split-right 50%    # half the window
# tmux: split-down 15      # 15 rows
```

`split` alone is an alias for `split-down`. The directions are spelled out rather than named
`-v`/`-h` on purpose: those mean **opposite** things in tmux (`-v` = below) and vim (`:vsplit` =
beside), so a short name would be ambiguous whichever convention it followed.

Always `chmod +x` after creating a task, or it will not appear.

## Choosing the placement

This is the decision that makes a task pleasant or annoying. Pick by lifetime, not by taste:

| placement | use for | behaviour |
| --- | --- | --- |
| `window` | anything slow or interactive — builds, test runs, REPLs | reuses a window named after the task (`android/build` → `android-build`), respawning it. Re-running never piles up duplicates. Gets selected. |
| `detach` | long-lived streams and fire-and-forget — `logcat`, watchers, deploys | same window reuse, but **not** selected. The status bar shows `●` while it runs and `✓`/`✗` when it ends. That marker *is* the notification — do not add `terminal-notifier`. |
| `popup` | quick checks under ~2s where only the exit code matters — `doctor`, a lint, a version probe | floats over the current window, no window created. Reuses the picker's own popup — tmux allows only one popup per client, so a second one would silently do nothing. |
| `split-down` / `split-right` | something you want beside the current work | splits the current window, 30% by default |

When unsure, use `window`. It is the default for a reason.

## Execution contract

Guaranteed for every task, whatever the placement:

- **cwd is the project root.** Scripts can assume `./gradlew`, `package.json`, etc. resolve.
  Never `cd` relative to the script's own location.
- `TMUX_TASK_ROOT` — the project root.
- `TMUX_TASK_NAME` — e.g. `android/build`.
- On exit the wrapper prints `✓ <name>` or `✗ <name> — exit <n>` and **waits for a keypress**.
  A fast failure never scrolls away. Do not add your own "press enter to continue".
- For `window` and `detach`, the wrapper also sets `@task_status` on the window — `running`,
  `ok` or `fail` — which the status bar renders as `●` / `✓` / `✗`. It persists until the task
  is re-run, so a failure is still visible when you come back to it. **The exit code is the
  whole signal, so a task must exit non-zero when it fails**; a script that swallows errors
  will report a green `✓`.

The root is resolved from tmux's `#{session_path}` — the session's working directory, **not**
the pane's. That is what makes the picker behave identically from a pane three directories
deep. Never write a task that depends on the pane's cwd.

## Checklist when adding a task

1. Confirm the project root — `tmux display-message -p '#{session_path}'`.
2. Create `<root>/.tmux/<group>/<name>` (or `<root>/.tmux/<name>` if ungrouped).
3. Shebang, `# task:` description, `# tmux:` placement, `set -euo pipefail`.
4. `chmod +x` it.
5. Verify it is discovered: `~/.local/bin/tmux-tasks --list`.
6. Only then tell the user it is ready.

Shared logic goes in `.tmux/lib/*.sh`, sourced via `"$TMUX_TASK_ROOT/.tmux/lib/common.sh"`, and
must stay non-executable.

## Surviving a new worktree

`.tmux/` is untracked, so a worktree created by a worktree manager starts without it and
`Prefix + e` will report no tasks there.

If the project uses grove, add a line to its `.grove/setup.sh` — which runs after a worktree is
created, with `GROVE_REPO_ROOT` and `GROVE_WORKTREE_PATH` set:

```bash
[ -d "$GROVE_REPO_ROOT/.tmux" ] && cp -R "$GROVE_REPO_ROOT/.tmux" "$GROVE_WORKTREE_PATH/.tmux"
```

`cp -R`, not the `cp_if_exists` helper those scripts usually define — that helper tests `[ -f ]`
and silently skips directories.

This is the only place the two systems touch, and the dependency points one way: tmux tasks know
how to survive a worktree; grove knows nothing about tmux tasks.

## Worked examples

```bash
# .tmux/android/logcat
#!/usr/bin/env bash
# task: stream logcat for the app process only
# tmux: detach
set -euo pipefail
adb logcat --pid="$(adb shell pidof -s com.example.app)"
```

```bash
# .tmux/shared/framework
#!/usr/bin/env bash
# task: build the KMP shared framework for the iOS simulator
# tmux: window
set -euo pipefail
./gradlew :shared:linkDebugFrameworkIosSimulatorArm64
```

```bash
# .tmux/doctor
#!/usr/bin/env bash
# task: check the toolchain is sane
# tmux: popup
set -euo pipefail
java -version
xcodebuild -version
adb devices
```

## Troubleshooting

| symptom | cause |
| --- | --- |
| task missing from picker | not executable, or nested deeper than 2 levels |
| `Prefix + e` flashes a message instead of opening | the project has no tasks; the binding gates on `tmux-tasks --check` so the popup never opens empty |
| duplicate windows on re-run | the task name changed, so the derived window name changed |
| task runs in the wrong directory | it `cd`s relative to itself instead of trusting the root |
| helper file shows up as a task | it is `chmod +x`; remove the executable bit |

Do not hand-edit the MRU cache in `$TMPDIR/tmux-tasks.<hash>` — it is rewritten on every run.
