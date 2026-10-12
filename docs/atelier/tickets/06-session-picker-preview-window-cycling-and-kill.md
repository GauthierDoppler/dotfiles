# 06: Session picker: preview, window cycling and kill

**What to build:** The rest of the session picker: preview of the highlighted session's current window, `h`/`l` to cycle that session's window, `Ctrl-x` to kill and keep picking. The bash session picker is deleted.

**Blocked by:** 05 (Session picker: list, scope and switch)

**Status:** done

- [x] Preview shows the window list and the tail of the capture, trailing blank lines dropped
- [x] h/l change the other session's current window, and Enter lands on it
- [x] Ctrl-x kills the session and the list refreshes
- [x] Bash script removed, `AGENTS.md` updated

## Comments

- The preview, `h`/`l` and `Ctrl-x` call back into `atelier sessions
  preview|next|prev|kill`; `tests/sessions.rs` drives each one against a private
  tmux server (window list, tail sized by `$FZF_PREVIEW_LINES`, styled-blank
  trailing lines dropped, cycling then Enter landing on that window, kill
  followed by fewer rows, placeholder no-ops). The fzf binds themselves were
  checked by hand with fzf 0.65 in a tmux pane on Linux, not by a test.
- `Ctrl-x` is a `transform` that kills and prints `reload(rows)`, like `Tab`,
  rather than the bash `execute-silent(...)+reload(...)` pair.
- The `←` marking the current window is appended by atelier rather than put in
  the `list-windows` format: tmux turns it into `_` for a client without a UTF-8
  locale.
- `h`/`l` on a single-window session do nothing instead of swallowing tmux's
  error; any other tmux error is reported.
- The `unbind(esc)` note that lived in `scripts/tmux-sessions` moved into the
  header of `scripts/tmux-tasks`, which pointed at it.
- Not verified on macOS.
