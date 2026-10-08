# Atelier

## Problem Statement

My workstation runs on tmux sessions made by grove, per-project tasks under
`.tmux/`, a status bar that tells me which project and checkout I am in, and a
markdown preview I use to review and annotate what agents write. All of it works,
and all of it is about 1,600 lines of bash that grew one fix at a time.

- **Nothing is tested.** Most of the hard-won knowledge is a paragraph in
  `AGENTS.md` describing a bug that was fixed once. Nothing stops it coming back.
- **Every script works things out from nothing on every call.** The status bar
  starts `git` and `pmset` every second whatever is happening. The left and right
  blocks ask each other for their widths.
- **Grove's naming is reverse-engineered.** The session picker and the status bar
  each carry a copy of grove's FNV-1a session key to guess which project a session
  belongs to. If grove changes its naming, both silently go wrong.
- **It is tied to one Mac.** `pmset`, launchd plists, JXA and BSD tool quirks are
  spread through the scripts. A remote Linux machine is coming and should get the
  same environment.

## Solution

One Rust binary, `atelier`, that orchestrates the workstation. It is not a rewrite
of everything: config files stay plain files, grove stays a separate TypeScript
tool, Homebrew still installs packages.

- Grove tags each session it creates with tmux options, and atelier reads them.
  No more name parsing.
- The status bar, the session picker, the task picker and the token picker become
  atelier commands. fzf stays as the picker UI.
- A daemon started by tmux keeps the state in one place and pushes the rendered
  bar into tmux options when something changes, instead of tmux running scripts
  every second.
- The markdown preview server moves into atelier, with the same page, the same
  notes and a way to open it without nvim.
- `atelier setup`, `atelier service` and `atelier doctor` replace the install
  logic and the hand-written launchd plists, and work on macOS and Linux.
- If atelier is broken, the machine still works: shell, git, tmux and grove do
  not depend on it, the bar falls back to the session name, tasks still run.

## User Stories

### Sessions and grove

1. As the workstation owner, I want each grove session to carry its project, repo root and worktree as tmux options, so that every tool reads the same answer instead of guessing from the session name.
2. As the workstation owner, I want sessions I named by hand to still show a sensible project, so that a session outside grove does not break the bar or the picker.
3. As the workstation owner, I want the session picker to list this project's sessions first and toggle to all sessions with `Tab`, so that I switch worktrees of one project without scanning everything.
4. As the workstation owner, I want the session picker preview to show the tail of the session's current window, so that I see the prompt, the last command or the error at a glance.
5. As the workstation owner, I want to cycle another session's current window from the picker, so that I can watch what it is doing and attach straight to that window.
6. As the workstation owner, I want to kill a session from the picker and keep picking, so that I can clean up finished worktrees in one pass.
7. As the workstation owner, I want the picker to open even when there is no other session, so that `Tab` to other projects is always reachable.

### Status bar

8. As the workstation owner, I want the left of the bar to show the project and whether I am at the root or in a worktree, so that I always know which checkout I am editing.
9. As the workstation owner, I want the right of the bar to show lines changed, commits ahead and behind, battery, date and clock, so that repo state is visible without running git.
10. As the workstation owner, I want the window list to stay centred whichever session is attached, so that windows do not jump when I switch.
11. As the workstation owner, I want narrow clients to drop whole segments (date, then battery, then repo counts) rather than the window list, so that the most useful part survives on a small screen.
12. As the workstation owner, I want a window to occupy the same width whether it is active or not, so that focusing a window never shoves the others.
13. As the workstation owner, I want repo counts to update right after a commit, checkout or edit, so that the bar reflects reality without a polling delay.
14. As the workstation owner, I want nothing to run every second when nothing changes, so that the bar costs nothing while I read.
15. As the workstation owner, I want the bar to fall back to the session name when atelier is not running, so that a broken atelier never leaves an empty bar.

### Tasks

16. As the workstation owner, I want any executable file under a project's `.tmux/` to appear in the task picker with its description, so that adding a task is just writing a script.
17. As the workstation owner, I want subfolders of `.tmux/` to show as groups I cycle with `Tab`, so that a project with many tasks stays navigable.
18. As the workstation owner, I want tasks ordered by most recently run, so that the loop I am iterating on is always first.
19. As the workstation owner, I want each task to run from the project root with its root and name in the environment, so that it behaves the same from any pane.
20. As the workstation owner, I want a task to open in a window, a split, a popup or detached as its header says, so that each loop gets the placement it needs.
21. As the workstation owner, I want a window or detached task to reuse its window instead of piling up duplicates, so that re-running is cheap.
22. As the workstation owner, I want the task's window to show running, succeeded or failed, so that I know the outcome without looking at the output.
23. As the workstation owner, I want a bell when a window or detached task finishes while I am elsewhere, so that I come back at the right time.

### Claude Code

24. As the workstation owner, I want the window running Claude Code to show a yellow marker when it waits for me and a green one when it is done, so that I know which session needs me.
25. As the workstation owner, I want the marker cleared when I go to the window, so that going to look is what dismisses it.
26. As the workstation owner, I want an optional notification when Claude waits and I am not looking, that opens the right session, window and pane when clicked, so that I can leave the terminal without losing track.

### Picking out of a pane

27. As the workstation owner, I want to fuzzy-pick a URL or file path from the current pane and open, copy or hand it to the OS, so that I never select text by hand for those.
28. As the workstation owner, I want truncated path fragments filtered out, so that every candidate actually exists.

### Markdown preview

29. As the workstation owner, I want to open a markdown file's preview in the browser, reusing its tab if one is open, so that I do not collect duplicate tabs.
30. As the workstation owner, I want the preview to re-render on save and follow my nvim cursor, so that I read what I am editing.
31. As the workstation owner, I want to annotate line ranges in the margin and copy them all as `path` plus `Lx-Ly: comment` lines, so that I can paste a review into an agent.
32. As the workstation owner, I want my existing notes to still be there after the rewrite, so that no review is lost.
33. As the workstation owner, I want to pick a markdown file of the current session from a tmux popup and preview it, so that I can review an agent's plan without opening nvim.
34. As the workstation owner, I want the preview to refuse to serve arbitrary local files to arbitrary pages, so that rendering untrusted markdown cannot read my disk.

### Setup and machines

35. As the workstation owner, I want one bootstrap command on a fresh Mac or Linux machine, so that a new machine is ready without following notes.
36. As the workstation owner, I want a `desktop` and a `remote` profile, so that a headless Linux box does not get GUI apps or the keyboard layout.
37. As the workstation owner, I want re-running setup to change nothing when nothing changed, so that I can run it any time without fear.
38. As the workstation owner, I want machine-local additions to `~/.zshrc`, `~/.zprofile` and `~/.gitconfig` kept below the load line and listed by `local-diff`, so that I decide what to promote.
39. As the workstation owner, I want Claude Code's settings merged from the shared base, my local overrides and what the app changed, so that neither side's changes are lost.
40. As the workstation owner, I want to describe a background service once and get a launchd agent on macOS or a systemd unit on Linux, so that cc-tap runs the same way on both.
41. As the workstation owner, I want `atelier doctor` to check terminfo for my terminal, the Nerd Font, the tmux version, linger and services, and print the fix for each failure, so that gotchas are caught instead of remembered.
42. As the workstation owner, I want none of this to assume a specific terminal app, so that I can use ghostty, kitty, WezTerm or iTerm2.

## Implementation Decisions

- **Language and distribution.** Rust, one binary installed with `cargo install`
  from the crate inside the dotfiles repo. No release pipeline: each machine builds
  it.
- **Orchestrator boundary.** Atelier owns behaviour (bar, pickers, task runner,
  preview server, setup, services, doctor). Config files stay plain files linked
  from the repo. Grove stays in its own repo and language. Homebrew stays the
  package installer.
- **The contract with tmux is options.**
  - `@grove_project`, `@grove_root`, `@grove_worktree` on sessions, set by grove.
    `@grove_worktree` is empty at the main checkout. Grove also sets them when
    attaching to an existing session, so old sessions get tagged.
  - `@task_status` (`running`/`ok`/`fail`) and `@claude_status`
    (`waiting`/`done`) on windows, written directly by the CLI. Writing a marker
    never needs the daemon.
  - `@bar_left` and `@bar_right` on sessions, written by the daemon. The tmux
    config shows them and falls back to calling atelier through `#()` when they are
    unset.
- **Session resolution.** Grove options first. Otherwise git from the session's
  path (main worktree as root). Otherwise the session name. One resolver used by
  the bar and the pickers.
- **Rendering.** Left and right blocks are pure functions of a state snapshot and
  the client width. Width is counted in terminal cells, independent of the locale.
  Truncation happens in atelier, never through tmux's length options. The clock
  stays in tmux (`%H:%M`, fixed width) so the daemon needs no per-minute timer.
- **Daemon.** Started by the tmux config with an "ensure" flag, one per tmux
  socket, exits when tmux exits. A single control-mode connection is its only link
  to tmux: it learns about sessions, windows, focus and resizes from events and
  sends `set-option` on the same connection. It watches each in-use repo's index,
  HEAD and refs instead of polling. Every "is someone looking at this window"
  check ignores control-mode clients.
- **CLI ↔ daemon.** JSON lines over a Unix socket in the runtime directory. Every
  client falls back to talking to tmux directly when the daemon is down.
- **Pickers.** fzf stays the UI. Atelier prints rows and handles fzf's callbacks.
  Placeholder rows when there is nothing to list. The `.tmux/` task contract does
  not change: executable bit, depth 1–2, `# task:` and `# tmux:` headers in the
  first 20 lines.
- **Markdown preview.** Same browser page and vendored libraries, embedded in the
  binary. Same routes and same notes directory, so existing notes keep working.
  The server runs as its own long-lived process (`atelier preview serve`) under the
  service manager, not inside the tmux daemon, so it works without tmux. Security
  fences are kept as they are: sanitised HTML, script-src self CSP, Host check
  against DNS rebinding, Referer rule for non-markdown files, JSON-only writes.
  "Copy all" stays the way to hand notes to an agent; nothing writes into a Claude
  pane.
- **Services.** One description file maps to launchd plists or systemd user units
  and timers. A plist or unit is only reloaded when it changed. On Linux, setup
  enables linger so services survive SSH logout.
- **Clipboard and opening.** Copy goes through OSC 52 via tmux, so it reaches the
  local clipboard over SSH too. Chrome tab reuse is macOS only; elsewhere the
  preview prints the URL.
- **Terminal agnostic.** Atelier never names a terminal app. When it needs one
  (bringing it to the front after a notification click), it walks up the process
  tree from the tmux client's pid.
- **Notifications** are last and optional, local to the Mac, for `waiting` only,
  once per wait, never when a focused client shows the pane.

## Testing Decisions

A good test drives atelier the way tmux and I do, and checks what I would see: a
tmux option's value, the rows fzf would show, an HTTP response, the files setup
leaves behind. Tests do not reach into internal modules or assert on how state is
stored.

Seams, highest first:

1. **A private tmux server plus the atelier binary.** The main seam. Each test
   starts its own server on a random socket with an empty config, sets up
   sessions, windows and options, runs atelier commands or the daemon, then reads
   tmux back. Covers the resolver, pickers (rows and callbacks), the task runner's
   placements and markers, and the daemon's pushes.
2. **The render functions.** Pure, snapshot-tested. Justified as a second seam
   because the matrix (width tiers × marker states × focus × segment
   combinations) is too large to drive through tmux, and because the width bugs
   recorded so far all lived here.
3. **HTTP against the preview server** on a temporary directory. Covers routes,
   notes persistence and every security fence.

Setup is tested at seam 1's level of abstraction: run it against a temporary
`$HOME` and inspect the result, twice, to prove it is idempotent.

Prior art in the repo is thin: the headless nvim smoke test is the only existing
test. CI runs on Linux and macOS, plus shellcheck on the bash that remains.

## Out of Scope

- Rewriting grove, or teaching grove anything about `.tmux/`.
- Nix or home-manager.
- Notifications from the remote machine to the Mac.
- Writing annotations into a Claude Code pane.
- Session persistence of any kind.
- A Kotlin language server or debugger in nvim (unchanged decision).

## Further Notes

- Each ticket deletes the bash it replaces and updates `AGENTS.md`. Paragraphs
  that only record a fixed bug shrink to a pointer at the test that now covers it.
- Bringing notifications back reverses a decision recorded in `AGENTS.md`; that
  section is rewritten when the notification ticket lands.
- Tickets live in `tickets/`, one file each, numbered in dependency order. Work
  the frontier: any ticket whose blockers are done.
