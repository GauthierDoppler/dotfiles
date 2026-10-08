# 09: Claude Code marker through atelier

**What to build:** Claude Code's hooks call `atelier hook claude`, which sets the window's waiting or done marker and clears it on prompt submit. Replaces the shell hook.

**Blocked by:** 02 (Atelier tracer bullet: the bar's project name comes from atelier)

**Status:** ready-for-agent

- [ ] Event taken from the hook payload, one command for all three hook entries
- [ ] Marker not set when its window is the active one of an attached, non-control-mode client
- [ ] Marker cleared when the window is selected
- [ ] Tested through the harness by feeding hook payloads
- [ ] Claude settings point at the new command, old hook removed
