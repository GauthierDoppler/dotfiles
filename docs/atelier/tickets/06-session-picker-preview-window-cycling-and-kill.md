# 06: Session picker: preview, window cycling and kill

**What to build:** The rest of the session picker: preview of the highlighted session's current window, `h`/`l` to cycle that session's window, `Ctrl-x` to kill and keep picking. The bash session picker is deleted.

**Blocked by:** 05 (Session picker: list, scope and switch)

**Status:** ready-for-agent

- [ ] Preview shows the window list and the tail of the capture, trailing blank lines dropped
- [ ] h/l change the other session's current window, and Enter lands on it
- [ ] Ctrl-x kills the session and the list refreshes
- [ ] Bash script removed, `AGENTS.md` updated
