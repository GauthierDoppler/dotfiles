# 16: Markdown picker on Prefix + m

**What to build:** A tmux popup lists the session's markdown files, most recently changed first, and opens the chosen one in the preview. Reviewing an agent's plan no longer needs nvim.

**Blocked by:** 14 (Markdown preview served by atelier)

**Status:** done

- [x] Files under the session root, gitignored ones excluded
- [x] Placeholder row when there is none
- [x] The token picker gets a preview action on markdown paths

## Comments

- `atelier preview pick` (fzf, for `display-popup -E`) and `atelier preview list`
  (its rows), in `atelier/src/preview/picker.rs`. Rows are paths relative to
  `#{session_path}`, newest mtime first. Inside a repo: `git ls-files -co
  --exclude-standard`; outside: a walk skipping dot-directories, six levels deep
  at most so a session rooted at `$HOME` stays usable. Placeholder row
  `(no markdown file under this session)`; `atelier preview` on it is a no-op.
- Enter is fzf `become(atelier preview {})`, run with fzf's cwd at the session
  root. On Linux the preview only prints the URL, which vanishes with the popup;
  on macOS it focuses or opens the Chrome tab. The macOS path is unverified here.
- Token picker: `Ctrl-v` → `atelier pick preview`, which previews a markdown
  path resolved from the pane's directory and does nothing on URLs, other files
  and the placeholder.
- `Prefix + m` overrides tmux's default `m` (mark pane), unused in this setup.
  The cheat sheet popup grew to 25 rows to fit the new line.
- The fzf launch flags are copied from `pick.rs` rather than shared, to keep this
  ticket off `pick.rs`'s picker code while other tickets land; worth folding into
  one helper when the pickers settle.
