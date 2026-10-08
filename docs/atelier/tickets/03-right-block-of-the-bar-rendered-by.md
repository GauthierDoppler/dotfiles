# 03: Right block of the bar rendered by atelier

**What to build:** The whole right block (key table, repo counts, battery, date, clock) is produced by atelier with exactly the same look as today, and the bash right-block script is deleted.

**Blocked by:** 02 (Atelier tracer bullet: the bar's project name comes from atelier)

**Status:** ready-for-agent

- [ ] Snapshot tests for every width tier, with and without repo counts and battery
- [ ] Repo counts use lines changed against HEAD and ahead/behind the upstream, without taking git's index lock; no upstream, detached HEAD and an empty repo are handled
- [ ] Battery works on macOS and Linux, and the segment disappears on a machine with no battery
- [ ] Width is counted in terminal cells and does not depend on `LANG`
- [ ] Before deletion, old and new output are compared at 80, 100, 120 and 200 columns for the same repo state
- [ ] Bash script removed, `AGENTS.md` status-bar section updated
