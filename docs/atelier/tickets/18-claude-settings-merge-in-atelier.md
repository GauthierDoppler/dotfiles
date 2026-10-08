# 18: Claude settings merge in atelier

**What to build:** The 3-way merge of Claude Code settings (shared base, local overrides, app drift, snapshot) moves into atelier with tests, and the bash version is deleted.

**Blocked by:** 17 (Setup: links and stubs)

**Status:** ready-for-agent

- [ ] Same semantics: app drift captured into local, base changes propagate where the app was silent
- [ ] A table of merge cases as tests, including key reordering and absolutised paths
- [ ] `--dry-run` shows what would be captured
- [ ] Bash script removed
