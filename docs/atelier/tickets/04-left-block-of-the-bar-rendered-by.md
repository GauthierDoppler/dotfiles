# 04: Left block of the bar rendered by atelier

**What to build:** Project and root/worktree pills produced by atelier, padded to the right block's width so the window list stays centred. The bash left-block script and its copy of grove's session key are deleted.

**Blocked by:** 01 (Grove tags its sessions), 03 (Right block of the bar rendered by atelier)

**Status:** ready-for-agent

- [ ] Snapshot tests: short and long project names, root vs worktree, a non-grove session, every width tier
- [ ] Padding equals the right block's width above the widest tier and stops below it
- [ ] Long project names truncate with an ellipsis without changing the block width
- [ ] Old and new output compared before deletion
- [ ] Bash script removed, `AGENTS.md` updated
