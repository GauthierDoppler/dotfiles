# 17: Setup: links and stubs

**What to build:** `atelier setup` replaces the linking, stub and migration logic of the install script, with `desktop` and `remote` profiles and a dry run. The install script shrinks to a bootstrap.

**Blocked by:** 02 (Atelier tracer bullet: the bar's project name comes from atelier)

**Status:** done

- [x] Same links and stub contents as today on the desktop profile
- [x] Existing files backed up as today, legacy local files migrated as today
- [x] Running twice changes nothing, tested against a temporary home directory
- [x] `--dry-run` prints the plan and touches nothing
- [x] The remote profile skips the keyboard layout, GUI app configs and casks

## Comments

Done on branch `atelier/17-setup`.

- `atelier setup [--profile desktop|remote] [--dry-run] [--repo PATH]` in
  `atelier/src/setup.rs`; the `LINKS` and `STUBS` tables there are the one list.
  `install.sh` builds atelier (now fatal on failure) and calls it where it used
  to link and stub. Tests: `atelier/tests/setup.rs`, 15 cases against a
  temporary `$HOME`, including a second run that must leave every inode and
  mtime unchanged.
- Deviations from the bash: an existing `.bak` is never overwritten (the next
  free `.bak.N` is used; bash overwrote a file backup and nested a folder backup
  inside the old one), same for `.migrated`. Only changes are printed, and a
  clean run prints `setup: up to date`. `--repo` and the git-root fallback must
  look like this repo (`dot_zshrc` + `atelier/Cargo.toml`), so running it from
  another project fails instead of linking that project into `$HOME`.
- Default profile: `desktop` on macOS, `remote` elsewhere. The remote profile
  skips ghostty and zed in atelier; the keyboard layout and Brewfile casks are
  skipped in `install.sh` (`grep -v '^cask ' Brewfile | brew bundle --file=-`).
  Launch agents, `xcode-select`, `open` and `lsregister` are gated on Darwin.
- Scripts in `~/.local/bin` stay linked on remote: tmux calls them and they
  degrade without `pmset`. `cc-tap-service` (launchctl) waits on ticket 20,
  `ssh-setup` (pbcopy) on ticket 22.
- Unverified: `install.sh` was only shellchecked, not run, on either OS (no brew
  in the container; `brew bundle --file=-` reading stdin is per Homebrew's docs).
  The Rust tests ran on Linux only; CI covers macOS. A manual run of the binary
  against a scratch `$HOME` was blocked by the sandbox; the tests do the same
  through the built binary.
- Branches still editing the old link lines in `install.sh` (scripts deleted by
  tickets 03–16) must edit `LINKS` in `setup.rs` and `DESKTOP_LINKS` in
  `tests/setup.rs` instead.
