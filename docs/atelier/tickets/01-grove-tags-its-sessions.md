# 01: Grove tags its sessions

**What to build:** Every session grove creates or attaches to carries its project, repo root and worktree as tmux options, so other tools read them instead of decoding the session name. Work happens in the grove-ai repo.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] Creating a session sets `@grove_project`, `@grove_root` and `@grove_worktree` on it
- [ ] `@grove_worktree` is empty for the main checkout
- [ ] Attaching to a session that predates this change sets the three options
- [ ] Covered by grove's own test suite
