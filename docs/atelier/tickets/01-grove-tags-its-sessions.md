# 01: Grove tags its sessions

**What to build:** Every session grove creates or attaches to carries its project, repo root and worktree as tmux options, so other tools read them instead of decoding the session name. Work happens in the grove-ai repo.

**Blocked by:** None (can start immediately)

**Status:** done

- [x] Creating a session sets `@grove_project`, `@grove_root` and `@grove_worktree` on it
- [x] `@grove_worktree` is empty for the main checkout
- [x] Attaching to a session that predates this change sets the three options
- [x] Covered by grove's own test suite

## Comments

Implemented in grove-ai on branch `claude/wizardly-shannon-1i1h5t` (commit `3541856`). Tagging on attach is best-effort: a running session whose worktree is gone attaches untagged, as before.
