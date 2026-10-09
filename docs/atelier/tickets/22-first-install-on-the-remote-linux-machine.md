# 22: First install on the remote Linux machine

**What to build:** Split the Brewfile into shared and macOS-only parts, then bootstrap the remote machine from a fresh clone with the remote profile. Every manual step found becomes a setup step or a doctor check.

**Blocked by:** 18 (Claude settings merge in atelier), 19 (local-diff in atelier), 21 (atelier doctor)

**Status:** needs-human

- [ ] Fresh clone to working shell, tmux, grove, tasks and bar with one command
- [ ] `atelier doctor` passes
- [ ] No step done by hand remains undocumented
