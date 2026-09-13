# Pi resources

Tracked reusable resources for the Pi coding agent.

What is tracked here:

- `extensions/subagents/` — custom subagent extension.
- `agents/*.md` — user-level reusable subagent definitions.

Subagent usage examples:

```text
/subagent-run --tmux scout inspect the current repo
/subagent-run --tmux --background scout investigate routing engines
/subagent-run --headless --model openai-codex/gpt-5.5 planner draft an implementation plan
```

The model-facing `subagent` tool supports the same concepts with `runner`,
`model`, `thinking`, and `background` parameters.

What deliberately stays machine-local:

- `~/.pi/agent/settings.json` — provider/model/theme/package preferences.
- `~/.pi/agent/auth.json` — credentials.
- `~/.pi/agent/sessions/` — session history.
- `~/.pi/agent/npm/` and `~/.pi/agent/git/` — installed package caches.

`install.sh` links the extension directory and each agent file individually so other local Pi resources are not hidden.
