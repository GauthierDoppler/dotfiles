# 05: Session picker: list, scope and switch

**What to build:** `Prefix + Space` opens a picker fed by atelier: this project's sessions first, `Tab` toggles to all sessions, `Enter` switches. Grouping uses grove's options.

**Blocked by:** 01 (Grove tags its sessions), 02 (Atelier tracer bullet: the bar's project name comes from atelier)

**Status:** ready-for-agent

- [ ] Rows tested through the tmux harness for one project, several projects and a lone session
- [ ] A placeholder row appears when there is no other session, and Enter on it does nothing
- [ ] A hand-named session never scopes the picker to a project that does not exist
- [ ] j/k/i/Esc/q behave as today
