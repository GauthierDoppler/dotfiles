# Atelier tickets

Tracer-bullet slices of [the spec](../spec.md), one file each, numbered in
dependency order. Work the frontier: any ticket whose blockers are all done.

| # | Ticket | Blocked by | Status |
| - | ------ | ---------- | ------ |
| [01](01-grove-tags-its-sessions.md) | Grove tags its sessions | — | ready-for-agent |
| [02](02-atelier-tracer-bullet-the-bar-s-project.md) | Atelier tracer bullet: the bar's project name comes from atelier | — | ready-for-agent |
| [03](03-right-block-of-the-bar-rendered-by.md) | Right block of the bar rendered by atelier | 02 | ready-for-agent |
| [04](04-left-block-of-the-bar-rendered-by.md) | Left block of the bar rendered by atelier | 01, 03 | ready-for-agent |
| [05](05-session-picker-list-scope-and-switch.md) | Session picker: list, scope and switch | 01, 02 | ready-for-agent |
| [06](06-session-picker-preview-window-cycling-and-kill.md) | Session picker: preview, window cycling and kill | 05 | ready-for-agent |
| [07](07-task-picker-and-window-tasks.md) | Task picker and window tasks | 02 | ready-for-agent |
| [08](08-task-placements-split-popup-and-detach.md) | Task placements: split, popup and detach | 07 | ready-for-agent |
| [09](09-claude-code-marker-through-atelier.md) | Claude Code marker through atelier | 02 | ready-for-agent |
| [10](10-token-picker.md) | Token picker | 02 | ready-for-agent |
| [11](11-daemon-tracer-bullet-atelier-knows-the-live.md) | Daemon tracer bullet: atelier knows the live tmux state | 02 | ready-for-agent |
| [12](12-daemon-pushes-the-bar.md) | Daemon pushes the bar | 04, 11 | ready-for-agent |
| [13](13-repo-counts-follow-git-without-polling.md) | Repo counts follow git without polling | 12 | ready-for-agent |
| [14](14-markdown-preview-served-by-atelier.md) | Markdown preview served by atelier | 02 | ready-for-agent |
| [15](15-markdown-notes-and-cursor-sync.md) | Markdown notes and cursor sync | 14 | ready-for-agent |
| [16](16-markdown-picker-on-prefix-plus-m.md) | Markdown picker on Prefix + m | 14 | ready-for-agent |
| [17](17-setup-links-and-stubs.md) | Setup: links and stubs | 02 | ready-for-agent |
| [18](18-claude-settings-merge-in-atelier.md) | Claude settings merge in atelier | 17 | ready-for-agent |
| [19](19-local-diff-in-atelier.md) | local-diff in atelier | 17 | ready-for-agent |
| [20](20-services-on-launchd-and-systemd.md) | Services on launchd and systemd | 15, 17 | ready-for-agent |
| [21](21-atelier-doctor.md) | atelier doctor | 17, 20 | ready-for-agent |
| [22](22-first-install-on-the-remote-linux-machine.md) | First install on the remote Linux machine | 18, 19, 21 | needs-human |
| [23](23-terminal-agnostic-tmux-config.md) | Terminal-agnostic tmux config | — | done |
| [24](24-remove-zed-and-unused-nvim-plugins.md) | Remove Zed and unused nvim plugins | — | needs-human |
| [25](25-spike-clickable-notification-from-a-daemon.md) | Spike: clickable notification from a daemon | 09 | needs-human |
| [26](26-waiting-notification-click-to-focus-the-pane.md) | Waiting notification, click to focus the pane | 25 | ready-for-agent |
| [27](27-shrink-agents-md.md) | Shrink AGENTS.md | 06, 08, 10, 13, 15 | ready-for-agent |
