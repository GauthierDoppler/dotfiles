# Atelier design notes

Long-form reasoning behind rules that [`AGENTS.md`](../../AGENTS.md) states in
one line. Read this before changing one of those rules; it is not needed for
everyday work. Test names are under `atelier/tests/` unless prefixed `src/`.

## Daemon and tmux control mode

**One control client is the daemon's only link to tmux:** `tmux -C
attach-session -f no-output,ignore-size`. tmux 3.4 has no session-less control
client — one started with no session prints `%exit` at once — so the daemon
attaches to an existing session and never creates one.

- Attaching bumps that session's `#{session_last_attached}`, which the session
  picker sorts by (newest first, then name), so the daemon attaches to whichever
  session already sorts first: the bump keeps the order
  (`daemon.rs::the_daemon_attaching_leaves_the_picker_order_alone`).
- With `detach-on-destroy on` (tmux's default, not this config's) killing that
  session sends `%exit` while the server lives on, so after `%exit` the daemon
  reattaches if any session is left and exits otherwise; the `session-created`
  hook brings it back
  (`daemon.rs::the_daemon_outlives_the_session_it_watches_through`).
- Being attached, it is a client like any other in `list-clients`,
  `#{session_attached}` and the `client-*` hooks — hence the rule that every "is
  someone looking" check skips `#{client_control_mode}` = 1.
- State is rebuilt from `list-sessions`, `list-windows -a`, `list-clients` on
  structural notifications; renames and window closes are applied in place.
- Replies are paired with requests in order, `%begin`/`%end` by command number,
  because a block's lines are not escaped and a window named `%end 1 1 1` is
  legal. Control mode prints a tab in format output as `_`, so fields are
  space-separated with the free-text one last
  (`src/daemon/control.rs::a_block_line_that_mimics_another_command_number_stays_output`).

**There is no resize notification for other clients.** `%layout-change` only
covers windows of the daemon's own session, and subscriptions are evaluated
against the daemon's client. So on attach the daemon sets `@bar_daemon` to its
client name and installs `client-resized[73]` → `display-message -c <itself>
atelier:client-resized`, which arrives as a bare `%message` (an `if-shell`
wrapper would wrap it in a `%begin`/`%end` block instead). `client-detached[73]`
unsets `@bar_daemon` and that hook when the detaching client is the daemon, which
is what drops the bar to its fallback when the daemon is killed.
`display-message`'s `-c` is not format-expanded, which is why the name is
written into the hook
(`bar_daemon.rs::resizing_the_client_re_renders_at_the_new_width_tier`,
`bar_daemon.rs::killing_the_daemon_leaves_the_bar_on_its_fallback`).

**Automatic rename is lazy.** tmux re-evaluates `automatic-rename` only when its
event loop wakes, so a fresh window can read `tmux` (the forked server, before
`exec`) until some unrelated command or output wakes it, and only then does
`%window-renamed` arrive. Daemon tests turn `automatic-rename` off.

**Pushing the bar.** For every session a non-control client is on, the daemon
renders at the width of its most recently active client (`#{client_activity}`)
— the option is per session, the width per client, so one has to win. It
re-renders after any structural notification, a rename, a
`%message atelier:client-resized` and a battery reading that changed (read once
a minute), and only sends `set-option` when a block changed. Markers need
nothing: they live in the window formats, which tmux evaluates itself.

The pushed right block (`right::pushed`) stops short of three things tmux does
better: the key-table slot stays a tmux format (`#{p10:…client_key_table…}`),
because control mode has no key-table notification and a subscription would
report the *daemon's* key table; the date goes in as the literal `%a %d %b`
(battery `%` doubled) and is shown through `#{T:@bar_right}`, so it turns over at
midnight with no timer; the clock is `%H:%M` in the config, after it.

## Repo counts without polling

`src/daemon/repos.rs` watches every repo some session's path is in — attached or
not — and drops it with the last such session. The repo is found on disk, not by
asking git: a `.git` directory, or a `.git` file whose `gitdir:` is
`<common>/worktrees/<name>` plus that dir's `commondir`.

Watched: the worktree's own git dir (`index`, `HEAD`), the common dir (`HEAD`,
`packed-refs`) and `refs/` recursively, and the directory of every tracked file —
an unstaged edit touches nothing under `.git`. Worktree events count only for
paths in the `ls-files` set, so untracked and ignored files never start git; the
set is re-listed when the index changes. On FSEvents the worktree root is watched
recursively instead, filtered the same way. `*.lock` and access events are
ignored (git's own reads would otherwise loop). Events settle for 200 ms, then the
repo's counts are dropped and recomputed lazily at the next push, only for
sessions a client is on. The session identity is cached by session fields plus
the repo found. A path that cannot be watched (inotify's `max_user_watches`, a
tracked directory deleted by hand) is logged once and retried at every settle
(`bar_daemon.rs::a_directory_that_cannot_be_watched_is_logged_once`).

Counts are `diff-index --shortstat HEAD` plus `rev-list --left-right --count
@{upstream}...HEAD` rather than one `status --porcelain=v2 --branch`, which
would cost a full worktree scan for the ahead/behind alone.

## Status bar layout

**Centring.** `status-justify centre` centres the window list in the space
*remaining* after `status-left` and `status-right`, not in the terminal:
measured on a 120-column client, growing the right block by 36 columns moved the
list 18 columns left — exactly half. Hence the left block pads to the right
block's width (`right::width`, not a copy of the tier table), and everything on
the right is fixed-width: the key-table slot stays reserved at rest, the date is
`%a %d %b` and never `%-d`, and the counts are padded on the left so they grow
away from the clock.

**Narrowing.** Segments are shed whole — date, then battery, then counts —
because a version that kept them rendered an 80-column client with no window
list at all. The left padding gives way earlier: it never leaves less than 36
columns between the blocks, so below twice the right block plus 36 (154 columns
with a battery, 134 without) the list drifts off centre instead of being
squeezed.

**Two blocks on the left.** Which repo and which checkout are two questions; a
flag glued onto the project name reads as part of the name. Padding sits
*outside* the capsules: an internally padded pill reads as an empty slab for a
short name. The checkout is `root`/`wt`, not the worktree's name, which is nearly
always a sanitized copy of the branch and changed width on every switch. The
branch itself is not shown: lazygit, the prompt and nvim already show it, and at
22 columns it was the widest field on the bar.

**Why not parse session names.** Grove builds names as
`{prefix}{project}_{branch}_{key}` with a sanitizer mapping `/[.\s:/@]/` to `_`
and leaving existing `_` alone, so `grove_my_repo_main_f2d1` cannot be split from
the left, its shape also matches a hand-named `api_perf_beef`, and a name freezes
the branch at creation.

**Colour.** The window list is greyscale so its two signals read: blue
(`#8caaee`) is "you are here", yellow/green/red are state. A green active-window
block once made a green `✓` invisible, and `#e5c890` on `#8caaee` is the same
low-contrast trap — which is why, on the active window, the marker is the
coloured *tail of the pill* (dark glyph on the state colour, closing cap taking
that colour). On inactive windows the marker sits one space after its own name
and four before the next, because centred between two windows its owner was
ambiguous. Right-hand segments are separated by spacing alone: dim `·` bullets
at `#626880` read as empty slots.

**Width parity.** The active format's two capsules are replaced by two plain
spaces in the inactive one, and the marker slot is +3 columns on both sides, so a
window has one width across all twelve combinations of state and focus.

## Claude Code marker instead of notifications

The marker replaced a `terminal-notifier` banner that failed in ways no fix
addressed: with several sessions running there was no telling which instance had
fired, the banner was not clickable back to the right window, and it asked you to
leave the terminal to learn something about the terminal.

Bells went too. The hook used to ring so ghostty's `bell-features` gave a dock
badge, and Claude Code's own notification channel (unset = `auto` =
`terminal_bell` outside iTerm) rang a second time: two indistinguishable beeps,
neither locating anything, only one gated on whether the window was in front. A
beep that cannot be attributed to a window stops being trusted.

The marker reuses the `@task_status` slot because that slot's width parity was
already solved; a second marker would have meant redoing the accounting over a
much larger matrix. `✻` is U+273B, East Asian Width N like `✓` and `✗`, so it is
one cell.

## Project jump

`local-diff` reports every `project` registration as a local addition on every
run. Teaching it to skip them was rejected: it is a plain line-set diff with no
knowledge of zsh syntax, and a diff tool that silently drops lines stops being
trustworthy about the ones it does report.

`set-titles on` with a `set-titles-string` resolving the project from
`#{session_path}` was rejected as more machinery for a property that only has to
be right at cold start. Its one advantage would be a tab title that follows the
attached session; as it stands, switching sessions with `Prefix + Space` leaves
the tab reading the first project, as a hand-renamed tab would. The status bar's
left block is what stays truthful.

## Markdown preview server

It used to be one server per file on a port hashed from the path, bumped on
collision and killed after 10 minutes without a client. Chrome suspends
background tabs, which drops SSE, so the server died under any tab left in the
background, and a bumped port changed the origin — which is what notes were keyed
on. One permanent server with the absolute path as the URL path fixes both and
lets relative images and links between `.md` files resolve natively.

## Neovim history

`nvim/` was a git submodule pointing at a `kickstart.nvim` fork until the fork
had diverged too far for upstream merges (its stylua workflow was still gated on
`github.repository == 'nvim-lua/kickstart.nvim'` and had never run). It was
folded in with `git subtree`; the fork's 444 commits are reachable, but pre-fold
commits carry un-prefixed paths, so `git log -- nvim/<file>` stops at the merge.
