# 19: local-diff in atelier

**What to build:** `atelier local-diff` lists what has accumulated below the load line of each stub. The bash version is deleted.

**Blocked by:** 17 (Setup: links and stubs)

**Status:** done

- [x] Same line-set diff and output format as today
- [x] Tested against a temporary home directory

## Comments

- `atelier local-diff` iterates setup's `STUBS` table, which gained `shared` (the
  tracked file) and `format` (`Shell` or `Git`); the load template now uses
  `{shared}` so the file name is written once. `STUBS` was reordered to
  `.zshrc`, `.zprofile`, `.gitconfig` so findings print in the old order.
- The hand-written `clap::Subcommand` impl that `setup` carried is now the
  `flags_only!` macro in `main.rs`, shared by both flag-only features.
- Output was compared byte for byte with the deleted script on a scratch home
  (zshrc, zprofile, gitconfig, Claude override, legacy files): identical except
  that bash 5.2 on Linux expands the `~` in `${1/#$HOME/~}` back to `$HOME`, so
  the old script printed full paths there. macOS's bash 3.2 printed `~`, which
  is what atelier prints everywhere.
- Deviations: the Claude override now lists `false` and `null` values (jq's
  `paths(scalars)` silently dropped them, so a local `"x": false` override was
  invisible); lines and legacy file names are sorted bytewise rather than by the
  locale's collation; the JSON diff no longer needs `jq`.
- `--repo` and `$DOTFILES` both still select the checkout, then the current git
  root, then `~/dotfiles`, through setup's `repo_root`.
- `local-diff` keeps working as an alias in `dot_zshrc`. Existing machines keep
  a dangling `~/.local/bin/local-diff` symlink, since `atelier setup` does not
  prune links it no longer manages; the alias shadows it in interactive zsh.
- Not run on macOS.
