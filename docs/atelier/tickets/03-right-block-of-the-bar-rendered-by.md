# 03: Right block of the bar rendered by atelier

**What to build:** The whole right block (key table, repo counts, battery, date, clock) is produced by atelier with exactly the same look as today, and the bash right-block script is deleted.

**Blocked by:** 02 (Atelier tracer bullet: the bar's project name comes from atelier)

**Status:** done

- [x] Snapshot tests for every width tier, with and without repo counts and battery
- [x] Repo counts use lines changed against HEAD and ahead/behind the upstream, without taking git's index lock; no upstream, detached HEAD and an empty repo are handled
- [x] Battery works on macOS and Linux, and the segment disappears on a machine with no battery (no real battery exercised, see Comments)
- [x] Width is counted in terminal cells and does not depend on `LANG`
- [x] Before deletion, old and new output are compared at 80, 100, 120 and 200 columns for the same repo state
- [x] Bash script removed, `AGENTS.md` status-bar section updated

## Comments

**Implementation.** `atelier bar right "#{session_path}" "#{client_width}" "#{client_key_table}"`
and `atelier bar right --width <client_width>`, in `atelier/src/bar/right.rs`
(pure render, unit and `insta` snapshot tests), `atelier/src/bar/battery.rs`
(`starship-battery`) and `git::repo_counts` (tested through the binary in
`atelier/tests/bar_right.rs`). `scripts/tmux-status-left` now asks
`~/.local/bin/atelier bar right --width` and still falls back to 59 when
atelier is absent.

**Old vs new comparison**, run before the bash was deleted, on the same repo
states, with `date` and `pmset` stubbed for the bash:

- Render level: 20 bash outputs at 80/100/120/200 columns (clean, edits only,
  ahead only, edits + ahead + behind; battery 87 %, 15 %, charging; `root` and
  `prefix` key tables) are kept verbatim as the expected values of
  `renders_exactly_what_the_bash_script_it_replaced_rendered`. They are equal
  byte for byte, widths included.
- Binary level: a harness ran both commands over 4 repo states × 4 widths × 2
  key tables in this container (no battery). All 32 rows are identical once the
  bash's empty 10-column battery slot is removed and the real date and time are
  normalised.

**Deviations from the bash, all deliberate:**

- *No battery, no slot.* The bash padded an empty 6 + 4 columns. The segment and
  its gap are now dropped, and `--width` reports the narrower block (49 at full
  width instead of 59), so centring still holds.
- *`diff-index` instead of `diff`.* Writing the "never rewrites the index" test
  showed that porcelain `git diff --shortstat HEAD` refreshes and rewrites a
  stat-dirty index even under `--no-optional-locks` (git 2.43), so the bash's
  claim was false. Plumbing `diff-index` does not rewrite it.
- *LANG.* The first capture of the bash in this container was wrong: the
  script's `LANG=en_US.UTF-8` default names a locale that does not exist here, so
  bash counted bytes and under-padded the battery by 3 columns. That is exactly
  the bug the AGENTS paragraph described. The golden values come from a run under
  `C.UTF-8`.
- The clock is still rendered by atelier, because the ticket asks for the whole
  block. Moving it into tmux belongs to the daemon (spec, Rendering).

**Unverified here:**

- Battery reading on real hardware. The container has no battery, and macOS is
  only type-checked with `cargo check --target aarch64-apple-darwin`, so IOKit
  never ran.
- The bar in a live attached tmux client. No TTY was available. The output
  format is byte-identical to what tmux already consumed from the bash.

With atelier absent the right block is empty, as it was with the script absent.
Existing machines keep a dangling `~/.local/bin/tmux-status-right` symlink,
because `install.sh` has no removal phase.
