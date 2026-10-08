# 17: Setup: links and stubs

**What to build:** `atelier setup` replaces the linking, stub and migration logic of the install script, with `desktop` and `remote` profiles and a dry run. The install script shrinks to a bootstrap.

**Blocked by:** 02 (Atelier tracer bullet: the bar's project name comes from atelier)

**Status:** ready-for-agent

- [ ] Same links and stub contents as today on the desktop profile
- [ ] Existing files backed up as today, legacy local files migrated as today
- [ ] Running twice changes nothing, tested against a temporary home directory
- [ ] `--dry-run` prints the plan and touches nothing
- [ ] The remote profile skips the keyboard layout, GUI app configs and casks
