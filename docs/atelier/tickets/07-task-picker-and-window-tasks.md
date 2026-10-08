# 07: Task picker and window tasks

**What to build:** `Prefix + e` lists tasks from the session's `.tmux/` folder through atelier and runs a `window` task end to end: project root as cwd, root and name in the environment, window reused by name, running/ok/fail marker, bell on completion when not watched.

**Blocked by:** 02 (Atelier tracer bullet: the bar's project name comes from atelier)

**Status:** ready-for-agent

- [ ] Discovery: executable files at depth 1 and 2 only, a non-executable helper never listed
- [ ] Headers read from the first 20 lines; missing headers fall back to defaults
- [ ] Most recently run first, groups cycled with Tab, placeholder row when empty
- [ ] Root resolved from the session's path, identical from a pane three directories deep
- [ ] Re-running reuses the window and resets the marker to running first
- [ ] All of the above tested through the tmux harness with fixture task folders
