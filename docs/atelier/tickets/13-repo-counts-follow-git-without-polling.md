# 13: Repo counts follow git without polling

**What to build:** The daemon watches the index, HEAD and refs of each repo in use and refreshes the counts when they change. Nothing runs while nothing changes.

**Blocked by:** 12 (Daemon pushes the bar)

**Status:** ready-for-agent

- [ ] A commit, a checkout or a staged edit updates the counts within a second
- [ ] Works inside a worktree, whose `.git` is a file
- [ ] Watches are dropped when no session uses the repo
- [ ] No git process starts while the repo is untouched
