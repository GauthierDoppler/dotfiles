# 13: Repo counts follow git without polling

**What to build:** The daemon watches the index, HEAD and refs of each repo in use and refreshes the counts when they change. Nothing runs while nothing changes.

**Blocked by:** 12 (Daemon pushes the bar)

**Status:** done

- [x] A commit, a checkout or a staged edit updates the counts within a second
- [x] Works inside a worktree, whose `.git` is a file
- [x] Watches are dropped when no session uses the repo
- [x] No git process starts while the repo is untouched

## Comments

- Unstaged edits: decided to watch the worktree too, since an edit to a tracked
  file changes nothing under `.git`. Only directories that hold a tracked file
  are watched (non-recursive inotify watches, so `node_modules` and other
  ignored trees cost nothing), and an event counts only when its path is in the
  `git ls-files` set, so untracked and ignored writes never start git. The set
  is re-listed when the index changes. Tested: an unstaged edit updates within
  a second; an untracked file and a file under an ignored dir start no git.
- "In use" means any session whose path is in the repo, attached or not, so
  switching between sessions of watched repos never needs git. Counts are
  dropped on a change and recomputed lazily, only for sessions a client is on.
- The repo is found on disk (`.git` dir, or `.git` file + `commondir`), not
  through git, so following sessions forks nothing. `GIT_DIR`-style setups and
  bare repos are not followed (no session sits in one).
- Beyond the list: `session::identify` also forked `git worktree list` and
  `git rev-parse` on every push for sessions without grove options; it is now
  cached per session fields plus the repo found. A `git init` in a session's
  directory changes the repo found and so re-resolves; any other change to a
  repo's layout (e.g. converting a checkout) is not picked up until the
  session's fields change or the daemon restarts.
- Seam for "no git": a logging `git` script first on the daemon's `PATH`
  (`GitLog` in `tests/bar_daemon.rs`), no status counter added.
- "Re-render only the sessions on that repo": the push still renders every
  watched session, but only the changed repo's counts are recomputed and only
  changed blocks are sent; rendering is pure and cheap.
- A rescan event (inotify queue overflow) marks every repo changed.
- Found in review: a branch switch that empties a tracked directory makes git
  remove and recreate it, which kills its inotify watch while the daemon still
  counted it as watched. A removed path is now forgotten and watches are
  reconciled after every settle; covered by
  `edits_are_still_seen_in_a_directory_a_checkout_removed_and_recreated`. A
  directory removed and recreated by hand with no index change stays unwatched
  until the next change in that repo.
- A failed `watch` (e.g. `max_user_watches` exhausted on a huge repo) is
  skipped silently: edits there are only picked up with the next git change.
- Unverified on macOS: the FSEvents path watches the worktree root
  recursively instead of per directory (`notify` restarts its FSEvents stream
  on every added path) and relies on canonicalised paths matching the ones
  FSEvents reports (`/private/var/...`). CI on macOS runs the same tests.
