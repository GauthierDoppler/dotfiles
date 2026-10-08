# 20: Services on launchd and systemd

**What to build:** One description file declares the background services. `atelier service install` writes launchd agents on macOS or systemd user units and timers on Linux. cc-tap's agents and the preview server move to it; hand-written plists are deleted.

**Blocked by:** 15 (Markdown notes and cursor sync), 17 (Setup: links and stubs)

**Status:** ready-for-agent

- [ ] Keep-alive services and the weekday 08:00 update are expressible
- [ ] A service is reloaded only when its generated file changed, and started if it is not running
- [ ] An unwritable LaunchAgents folder is reported with the command that fixes it
- [ ] On Linux, linger is enabled so services survive SSH logout
- [ ] Generated files snapshot-tested for both systems
