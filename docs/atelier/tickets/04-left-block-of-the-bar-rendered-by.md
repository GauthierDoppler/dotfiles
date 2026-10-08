# 04: Left block of the bar rendered by atelier

**What to build:** Project and root/worktree pills produced by atelier, padded to the right block's width so the window list stays centred. The bash left-block script and its copy of grove's session key are deleted.

**Blocked by:** 01 (Grove tags its sessions), 03 (Right block of the bar rendered by atelier)

**Status:** done

- [x] Snapshot tests: short and long project names, root vs worktree, a non-grove session, every width tier
- [x] Padding equals the right block's width above the widest tier and stops below it
- [x] Long project names truncate with an ellipsis without changing the block width
- [x] Old and new output compared before deletion
- [x] Bash script removed, `AGENTS.md` updated

## Comments

`atelier bar left -t <session> [client_width]` now prints the whole block
(`atelier/src/bar/left.rs`, pure render, snapshot-tested). The project and the
root/wt kind both come from `session::resolve`, which gained a `checkout` field:
`@grove_worktree` empty or not for a grove session, else `--git-dir` vs
`--git-common-dir` on `#{session_path}`, else none (single grey pill). The
right block's width is `right::width` called in-process; `bar right --width`
lost its only caller and was removed, and its test in `tests/bar_right.rs` now
checks the rendered width against itself across locales instead.

**Old vs new.** Recorded the bash (run against the pre-change binary) for four
sessions (outside a repo, root, linked worktree, 41-character name) at 60, 80,
90, 99, 100, 119, 120 and 200 columns; atelier's output is byte-identical in all
32 cases, both through the binary on a private tmux server and as the
`renders_exactly_what_the_bash_script_it_replaced_rendered` unit table. As in
ticket 03, the first capture was wrong: `en_US.UTF-8` does not exist in the
container, so bash counted the ellipsis as 3 columns and under-padded a long
name by 2. The golden values come from a run under `C.UTF-8`. The bash no longer
carried a copy of grove's FNV key by the time this ticket started, so there was
none to delete.

**Deviations:**

- `MAX_PROJECT` is counted in terminal cells, not characters, so a CJK name is
  clipped to 24 cells too.
- The padding does not match the right block "above the widest tier" (120): it
  gives way whenever `client_width < 2 × right + 36`, i.e. below 154 columns
  with a battery and 134 without. That is the bash's rule, kept as is; the
  snapshots show it, and AGENTS.md's claim that nothing engages above 120 was
  corrected.
- Likewise a long name keeps the block width only where the block pads to the
  right one; on a narrow client the block is as wide as its content.
- The fallback when atelier is missing or fails is `echo " "#{q:session_name}`
  inside the same `#()`: the plain session name, unstyled. Verified on a live
  attached client (tmux nested in tmux, status line captured) with and without
  the binary.

**Unverified here:** macOS (battery presence changes the right width, which the
snapshots cover with and without). Existing machines keep a dangling
`~/.local/bin/tmux-status-left` symlink; `atelier setup` has no removal phase.
