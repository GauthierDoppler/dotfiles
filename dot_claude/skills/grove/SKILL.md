---
name: grove
description: >-
  Set up or debug Grove — the git-worktree manager that gives each worktree its own tmux session
  (`~/Developer/perso/grove-ai`). Use this WHENEVER the user wants to configure worktrees for a
  project: "set up grove here", "add a grove config", "make new worktrees get my .env files",
  "configure the tmux windows for this repo", "copy untracked files into new worktrees", "change
  where worktrees are created", or when a worktree comes up broken — missing env files, wrong
  windows, a session that won't attach, or orphaned sessions. Also use when writing or editing
  `.grove/config.yaml` or `.grove/setup.sh`.
---

# grove

Grove manages git worktrees and gives each one its own tmux session. `grove` alone opens an
interactive cockpit; the subcommands are scriptable.

```bash
grove                      # interactive cockpit (default)
grove ls                   # worktrees + tmux session status
grove new <branch>         # create worktree + session (bases on origin/<branch> if it exists)
grove attach <branch>      # attach, creating the session if missing
grove delete <branch> [-f] # remove worktree and kill its session (-f: modified/untracked files)
grove kill <branch>        # kill the session, keep the worktree
grove prune                # kill orphaned sessions with no matching worktree
```

Cockpit keys: `j/k` move · `Enter` attach · `n` new · `d` delete · `D` force delete · `x` kill
session · `q` quit.

**Grove must be run from the repository root.** It refuses to run from inside a worktree.

## Configuration

Two optional files, deep-merged with the project file winning:

1. `~/.config/grove/config.yaml` — personal defaults across every repo.
2. `<repo-root>/.grove/config.yaml` — committed with the repo.

Nested blocks (`tmux`, `worktrees`) merge key-by-key, so a project can override one key without
discarding the rest. **Arrays (`template_files`, `default_windows`) replace wholesale** — a
project that sets `default_windows` discards the global list entirely, it does not append.

> The repo's README lists user-global config under "not yet ported". Verify it actually takes
> effect before relying on it; put settings in the project file if in doubt.

```yaml
tmux:
  session_prefix: "grove_"
  session_name_format: "{prefix}{project}_{branch}_{key}"
  default_shell: "zsh"

worktrees:
  path: "./.claude/worktrees/"     # relative -> repo root; absolute used as-is; ~ and $VAR expand
  setup_script: ".grove/setup.sh"

  default_windows:
    - name: "cc"
      command: "claude"
      focus: true
    - name: "dev"
      focus_pane: 1                # 1-based
      layout: "even-horizontal"
      panes:
        - command: "pnpm i"
        - command: "cd packages/backend && pnpm i"
          split: "vertical"        # vertical | horizontal

  template_files: []
```

`worktrees.path` affects **new** worktrees only. Existing ones are always found by their real
path from `git worktree list`, so changing it never orphans them.

## Keeping `.grove/` out of the repo

The user keeps grove setups untracked — they are personal tooling, not project config, and this
matters most on repos we do not own. Use the `git skip` alias (defined in `dot_gitconfig`), which
picks the right mechanism automatically:

```bash
git skip .grove/config.yaml .grove/setup.sh    # hide them
git unskip .grove/config.yaml                  # reverse it
```

- **File is untracked** → appended to `.git/info/exclude`, a repo-local ignore that is never
  committed and needs no change to the project's `.gitignore`.
- **File is already tracked** (someone committed `.grove/` upstream) → `update-index
  --skip-worktree`, so local edits stop showing up in `git status`.

Prefer `git skip` over editing `.gitignore`: on a client repo, adding personal tooling to a
tracked ignore file is a change to shared state.

One caveat for the tracked case: `--skip-worktree` can make a later `git pull` fail or behave
oddly if upstream modifies that same file. If a pull complains about `.grove/`, `git unskip` it,
pull, then `git skip` it again.

## Session naming — and why it matters beyond grove

Default format `{prefix}{project}_{branch}_{key}`, where `{key}` is an FNV-1a hash of the repo
root, always the **last** field. It is identical for every branch and worktree of one repo,
which is how `scripts/tmux-sessions` decides "sessions of this project".

Changing `session_name_format` so the key is no longer trailing **breaks the session picker's
project scoping.** Do not move or drop `{key}`.

## Getting files into new worktrees

A fresh worktree has only tracked files. Anything gitignored — env files, secrets, local
config — must be put there deliberately. Two mechanisms:

| | use for |
| --- | --- |
| `template_files` + `.grove/templates/` | files that are the *same* in every worktree and safe to commit as templates. Copied preserving permissions. |
| `.grove/setup.sh` | anything else — copying real (uncommittable) files from the main worktree, installing deps, running migrations |

`setup.sh` runs after the worktree is created, with `GROVE_WORKTREE_PATH`, `GROVE_BRANCH`,
`GROVE_SESSION_NAME` and `GROVE_REPO_ROOT` in the environment.

The house pattern (from `bp-api`):

```bash
#!/bin/bash
set -e

ROOT="$GROVE_REPO_ROOT"
WT="$GROVE_WORKTREE_PATH"

# Skip the copy when running on the main worktree — otherwise every cp is a self-copy.
if [ "$ROOT" = "$WT" ]; then
  exit 0
fi

cp_if_exists() { [ -f "$1" ] && cp "$1" "$2"; return 0; }

cp_if_exists "$ROOT/.env"                        "$WT/.env"
cp_if_exists "$ROOT/.claude/settings.local.json" "$WT/.claude/settings.local.json"
```

Two things to respect when extending it:

- **`cp_if_exists` is files-only** (`[ -f ]`). Passing a directory silently does nothing — no
  error, no copy, and `set -e` will not catch it. Copying an untracked *directory* needs a
  separate helper:

  ```bash
  cp_dir_if_exists() { [ -d "$1" ] && cp -R "$1" "$2"; return 0; }
  ```

  `mkdir -p` the destination's parent first if it is not at the worktree root.
- **The main-worktree guard is load-bearing.** Without it, `cp x x` on the main worktree either
  errors under `set -e` or truncates the file.

## Setting up a new project

1. `cd` to the repo root (not a worktree). Confirm with `git rev-parse --show-toplevel`.
2. Create `.grove/config.yaml`. Set `worktrees.path` if the default `worktrees/` is wrong for
   this repo — check whether that path is gitignored.
3. List the windows the user actually opens by hand; that is what `default_windows` should be.
4. If the repo has gitignored files a worktree needs, write `.grove/setup.sh`, `chmod +x` it,
   and point `worktrees.setup_script` at it.
5. `git skip .grove/config.yaml .grove/setup.sh` so the setup stays untracked.
6. Verify with a throwaway branch: `grove new tmp-check`, confirm the files and windows landed,
   then `grove delete tmp-check -f`.

Do not report it working until step 6 has actually run.

## Troubleshooting

| symptom | cause |
| --- | --- |
| grove refuses to start | run from inside a worktree; `cd` to the repo root |
| new worktree missing env files | `setup_script` not set, not executable, or `cp_if_exists` used on a directory |
| global config seems ignored | may not be ported yet — move the setting into `.grove/config.yaml` |
| project's windows replaced the global ones | expected: arrays replace, they do not merge |
| session picker no longer scopes to the project | `session_name_format` no longer ends in `{key}` |
| sessions linger after deleting worktrees | `grove prune` |
| `.grove/` shows up in `git status` | not skipped — `git skip .grove/config.yaml .grove/setup.sh` |
| `git pull` complains about `.grove/` | `--skip-worktree` on a tracked file upstream changed: `git unskip`, pull, `git skip` again |

Grove itself is a Bun/TypeScript project at `~/Developer/perso/grove-ai`. Rebuild and reinstall
the binary after pulling with `bun run install:cli`. Tests are `bun run test` (vitest) — **not**
`bun test`, whose runner is incompatible with `@effect/vitest`.
