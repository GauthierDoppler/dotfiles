# 05: Session picker: list, scope and switch

**What to build:** `Prefix + Space` opens a picker fed by atelier: this project's sessions first, `Tab` toggles to all sessions, `Enter` switches. Grouping uses grove's options.

**Blocked by:** 01 (Grove tags its sessions), 02 (Atelier tracer bullet: the bar's project name comes from atelier)

**Status:** done

- [x] Rows tested through the tmux harness for one project, several projects and a lone session
- [x] A placeholder row appears when there is no other session, and Enter on it does nothing
- [x] A hand-named session never scopes the picker to a project that does not exist
- [x] j/k/i/Esc/q behave as today

## Comments

- `atelier sessions pick` execs fzf; rows are `<session id><TAB><label>`
  (`--with-nth=2..`), so Enter switches by id and a placeholder row has an empty
  id. Every bind calls back into `atelier sessions rows|toggle|header|escape|switch
  -t <session>`; those callbacks are what `atelier/tests/sessions.rs` asserts.
  The j/k/i/q binds are static fzf options, checked by hand only (below).
- "Same project" is `session::resolve`'s new `root`: `@grove_root`, else the main
  worktree of `#{session_path}`, else none. A session with no root scopes to
  nothing: it lists everything, Tab prints no action, the header says
  `(no project)`. A hand-named session inside a repo now does scope to that repo,
  grove sessions included — the bash picker treated it as having no project.
- The scope is the session option `@atelier_sessions_scope` on the session the
  picker was opened from (bash used a global file in `$TMPDIR`), reset to
  `project` on every open.
- The tmux bindings pass no `-t`/`-c`: tmux 3.4 does not expand formats in a
  `display-popup` shell command (`'#{session_id}'` arrived literally), so `pick`
  asks tmux for the current session and client itself, as the bash did.
- `Prefix + Space` and `Prefix + s` now run the new picker. It has no preview, no
  h/l and no Ctrl-x until ticket 06, and the header only lists what works.
  `scripts/tmux-sessions` stays on disk and linked, unbound, for 06 to delete.
- Names are padded by `char` count, not terminal cells: a session name with wide
  characters misaligns the window counts.
- Observed, unchanged from the bash: Esc out of search keeps the filtered list
  (fzf's `disable-search` freezes it; `clear-query` first does not help).
- **Verified by hand** on Linux, tmux 3.4, fzf 0.60.3, through a real client in a
  nested tmux: open, Tab both ways, j/k, i + typing, Esc out of search, Esc/q to
  close, Enter switching, Enter on the placeholder. Not run on macOS. Debian's
  fzf 0.44 is too old (`transform` needs 0.45), as it was for the bash.
