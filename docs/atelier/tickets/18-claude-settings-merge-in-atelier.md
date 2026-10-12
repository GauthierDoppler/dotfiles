# 18: Claude settings merge in atelier

**What to build:** The 3-way merge of Claude Code settings (shared base, local overrides, app drift, snapshot) moves into atelier with tests, and the bash version is deleted.

**Blocked by:** 17 (Setup: links and stubs)

**Status:** done

- [x] Same semantics: app drift captured into local, base changes propagate where the app was silent
- [x] A table of merge cases as tests, including key reordering and absolutised paths
- [x] `--dry-run` shows what would be captured
- [x] Bash script removed

## Comments

- `atelier claude-settings-sync [--dry-run] [--repo PATH]`, in
  `atelier/src/claude_settings.rs`; the repo is found the same way as
  `atelier setup` (`--repo`, else the current repo root, else `~/dotfiles`). The
  old script's `$DOTFILES` variable is no longer read. `install.sh` calls it right
  after `atelier setup`, as it called the bash. It is not folded into
  `atelier setup` itself.
- The merge cases are 17 tests in `atelier/tests/claude_settings.rs`, one per
  case rather than a literal table, driving the binary against a temporary
  `$HOME` and repo. Every expected value is a worked example; before writing the
  Rust, the same tests were run against the bash script (through a temporary
  shim) and it agreed on all of them except the one below.
- Deliberate deviation: the snapshot is only rewritten when its content changes,
  so a second run touches no file (story 37). The bash rewrote it every run with
  identical content. Output files keep the `jq -S .` shape: sorted keys, two-space
  indent, trailing newline. Numbers are compared as serde_json does, so `1` and
  `1.0` differ where jq would call them equal; nothing in the settings relies on
  that.
- `jq` is no longer needed for this; it stays in the Brewfile for `local-diff`.
- Machines that already ran the old install keep a dangling
  `~/.local/bin/claude-settings-sync` symlink: `atelier setup` does not prune
  links it no longer manages. Remove it by hand.
- Not run on macOS (Linux container only); nothing here is platform-specific.
