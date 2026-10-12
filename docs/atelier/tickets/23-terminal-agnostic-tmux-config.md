# 23: Terminal-agnostic tmux config

**What to build:** Nothing in the tmux config or task notifications assumes ghostty.

**Blocked by:** None (can start immediately)

**Status:** done

- [x] Terminal features declared for ghostty, kitty, WezTerm, iTerm2 and plain xterm
- [x] Comments and the cheat sheet describe generic behaviour without naming ghostty
- [x] Task notifications no longer impersonate ghostty's bundle

## Comments

Verified on Linux with tmux 3.4: `dot_tmux.conf` sources into a fresh server
(`-f /dev/null`, then `source-file` three times) with no error, and
`terminal-features` ends with exactly one entry each for `xterm-ghostty`,
`xterm-kitty`, `wezterm` and `xterm-256color` after every reload.
`bash -n scripts/tmux-task-run` passes. The iTerm2 defaults (`RGB`, `usstyle`,
`sync`) were read from the tmux 3.4 binary's built-in table.

Not verified, no terminal to try on this machine:

- Undercurl and synchronized output in kitty and WezTerm through tmux.
- iTerm2 actually being detected via XTVERSION by the Mac's tmux.
- The `terminal-notifier` banner without `-sender`: it should show under
  terminal-notifier's own name and icon, and clicking it no longer focuses any
  terminal.

Behaviour change: plain `xterm-256color` used to get `usstyle:sync` and now gets
`RGB` alone. A terminal that reports `xterm-256color` but does support styled
underlines — WezTerm on its default `term`, or ghostty over SSH with its `ssh-env`
integration — now renders undercurl as a plain underline. The fix is that
terminal's own TERM and terminfo, which is what `atelier doctor` (21) checks.
