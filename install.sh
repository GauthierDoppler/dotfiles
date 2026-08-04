#!/bin/bash
# Bootstrap a macOS machine from scratch.
# Run from the dotfiles directory: ./install.sh
#
# First run (no SSH key):  generates key, copies pubkey, exits.
# Second run (key exists): installs everything.

set -euo pipefail

DOTFILES="$(cd "$(dirname "$0")" && pwd)"

warn() { echo "WARNING: $*"; }

# ─── Phase 0: SSH key ──────────────────────────────────────
if [[ ! -f "$HOME/.ssh/github" ]]; then
  echo "No GitHub SSH key found. Setting one up first..."
  echo ""
  "$DOTFILES/scripts/ssh-setup" github
  echo ""
  open "https://github.com/settings/ssh/new"
  echo "Add the SSH key to GitHub, then run this script again."
  exit 0
fi

# ─── Phase 1: Xcode Command Line Tools ─────────────────────
if ! xcode-select -p &>/dev/null; then
  echo "Installing Xcode Command Line Tools..."
  xcode-select --install
  echo "Re-run this script after the installation completes."
  exit 1
fi

# ─── Phase 2: Homebrew ─────────────────────────────────────
if ! command -v brew &>/dev/null; then
  echo "Installing Homebrew..."
  /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
fi

if [[ -f /opt/homebrew/bin/brew ]]; then
  eval "$(/opt/homebrew/bin/brew shellenv)"
elif [[ -f /usr/local/bin/brew ]]; then
  eval "$(/usr/local/bin/brew shellenv)"
fi

# ─── Phase 3: Brew Bundle ──────────────────────────────────
echo "Running brew bundle..."
brew bundle --file="$DOTFILES/Brewfile"

# ─── Phase 4: Oh My Zsh ────────────────────────────────────
if [[ ! -d "$HOME/.oh-my-zsh" ]]; then
  echo "Installing Oh My Zsh..."
  sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" "" --unattended
fi

# ─── Phase 5: (was git submodules) ─────────────────────────
# nvim/ used to be a submodule pointing at a kickstart.nvim fork. It is plain
# files in this repo now: the fork had diverged past the point where upstream
# merges were realistic, so the submodule was costing a two-step commit dance
# and a clone that broke whenever the submodule had not been pushed.

# ─── Phase 6: Symlinks ─────────────────────────────────────
link() {
  local src="$DOTFILES/$1"
  local dest="$2"
  mkdir -p "$(dirname "$dest")"
  if [ -L "$dest" ]; then
    rm "$dest"
  elif [ -d "$dest" ]; then
    echo "backup: $dest -> ${dest}.bak"
    mv "$dest" "${dest}.bak"
  elif [ -e "$dest" ]; then
    echo "backup: $dest -> ${dest}.bak"
    mv "$dest" "${dest}.bak"
  fi
  ln -s "$src" "$dest"
  echo "linked: $dest -> $src"
}

# macOS scans ~/Library/Keyboard Layouts/ at login and its input-source daemon
# does not reliably follow a symlink there, so the layout is copied.
copy_bundle() {
  local src="$DOTFILES/$1"
  local dest="$2"
  mkdir -p "$(dirname "$dest")"
  if [ -e "$dest" ] && diff -rq "$src" "$dest" &>/dev/null; then
    echo "copy ok: $dest"
    return 0
  fi
  if [ -L "$dest" ]; then
    rm "$dest"
  elif [ -e "$dest" ]; then
    echo "backup: $dest -> ${dest}.bak"
    rm -rf "${dest}.bak"
    mv "$dest" "${dest}.bak"
  fi
  cp -R "$src" "$dest"
  echo "copied: $dest <- $src"
}

# Creates a small REAL file that loads the tracked config, instead of symlinking.
#
# Why: third-party installers (git-ai, nvm, rbenv, conda...) append to ~/.zshrc
# and ~/.gitconfig directly. When those are symlinks into this repo, the append
# lands in a tracked file — machine-specific absolute paths staged into a repo we
# push. With a stub, the append goes below the load line and stays untracked.
#
# `git config --global` writes here too, which is now the correct outcome.
#
# MUST be idempotent: re-running install.sh has to leave everything accumulated
# below the load line untouched. That is the whole point, so the "already
# stubbed" check comes before any write.
stub() {
  local dest="$1" load="$2"
  mkdir -p "$(dirname "$dest")"

  # Match on the LAST line of the load block, not the first: for a gitconfig the
  # first line is a bare `[include]`, which any unrelated include would satisfy.
  # The last line carries the path, so it is unique to this stub.
  if [ -e "$dest" ] && [ ! -L "$dest" ] && grep -qxF "${load##*$'\n'}" "$dest" 2>/dev/null; then
    echo "stub ok: $dest (local content untouched)"
    return 0
  fi

  if [ -L "$dest" ]; then
    rm "$dest"                                   # migrating off the old symlink
  elif [ -e "$dest" ]; then
    echo "backup: $dest -> ${dest}.bak"
    mv "$dest" "${dest}.bak"
  fi

  {
    echo "# Loads the shared dotfiles config. Everything BELOW this block is"
    echo "# machine-local and stays out of the dotfiles repo — put local overrides"
    echo "# here, and let installers append here too."
    echo "$load"
    echo
  } > "$dest"
  echo "stubbed: $dest"
}

# One-time move of the old ~/.<name>.local files into their stub. Kept as
# .migrated rather than deleted: losing a machine's only copy of its local
# config to a convenience rename is not a recoverable mistake.
migrate_local() {
  local legacy="$1" dest="$2"
  [ -f "$legacy" ] || return 0
  {
    echo
    echo "# ─── migrated from $(basename "$legacy") ───"
    cat "$legacy"
  } >> "$dest"
  mv "$legacy" "${legacy}.migrated"
  echo "migrated: $legacy -> $dest (kept as ${legacy}.migrated)"
}

# ~/.config/* folders (whole directory symlinks)
link "delta"               "$HOME/.config/delta"
link "ghostty"             "$HOME/.config/ghostty"
link "lazydocker"          "$HOME/.config/lazydocker"
link "lazygit"             "$HOME/.config/lazygit"
link "nvim"                "$HOME/.config/nvim"
link "zed"                 "$HOME/.config/zed"

# ~/.config/* individual files
link "git/ignore"          "$HOME/.config/git/ignore"

# ~/ dotfiles (dot_ prefix becomes .)
link "dot_tmux.conf"       "$HOME/.tmux.conf"

# Keyboard layout (AZERTY with an unshifted number row — the tmux Prefix + 1..9
# bindings depend on it). Installing it does not SELECT it: that is a one-time
# manual step in System Settings → Keyboard → Input Sources. Writing
# AppleEnabledInputSources / AppleSelectedInputSources with `defaults` is cached
# by the input-source daemon, needs a logout to take, and half-works meanwhile.
copy_bundle "keyboard/FR-AZERTY-num.bundle" "$HOME/Library/Keyboard Layouts/FR-AZERTY-num.bundle"

# ~/.zshrc and ~/.gitconfig are stubbed, not linked — third-party installers
# append to them directly, and a symlink would put those appends in this repo.
# Written as $HOME / ~ rather than the absolute path, so the stub itself carries
# no machine-specific path. Built via variables: inside double quotes bash leaves
# `\~` as a literal backslash-tilde, which git then fails to parse.
# shellcheck disable=SC2016  # literal $HOME, expanded by zsh when it reads the stub
dollar_home='$HOME'
tilde='~'
stub "$HOME/.zshrc"     "source \"${DOTFILES/#$HOME/$dollar_home}/dot_zshrc\""
migrate_local "$HOME/.zshrc.local" "$HOME/.zshrc"

stub "$HOME/.gitconfig" "[include]
	path = ${DOTFILES/#$HOME/$tilde}/dot_gitconfig"
migrate_local "$HOME/.gitconfig.local" "$HOME/.gitconfig"

# Login-shell environment (locale, JAVA_HOME, Android SDK). Stubbed for the same
# reason as ~/.zshrc: installers append here too, and the env that accumulates is
# exactly the machine-specific kind.
stub "$HOME/.zprofile"  "source \"${DOTFILES/#$HOME/$dollar_home}/dot_zprofile\""

# ~/.claude/*
# settings.json is deliberately NOT linked: Claude Code rewrites it in place, so
# it is generated by claude-settings-sync below instead.
link "dot_claude/CLAUDE.md"            "$HOME/.claude/CLAUDE.md"
link "dot_claude/statusline-custom.sh" "$HOME/.claude/statusline-custom.sh"
link "dot_claude/hooks"                "$HOME/.claude/hooks"

# Skills are linked one by one, never as a whole directory: ~/.claude/skills
# also holds skills installed by Claude Code itself, and linking the parent
# would hide them.
mkdir -p "$HOME/.claude/skills"
link "dot_claude/skills/tmux-tasks"    "$HOME/.claude/skills/tmux-tasks"
link "dot_claude/skills/grove"         "$HOME/.claude/skills/grove"

# Scripts
mkdir -p "$HOME/.local/bin"
link "scripts/local-diff"            "$HOME/.local/bin/local-diff"
link "scripts/ssh-setup"             "$HOME/.local/bin/ssh-setup"
link "scripts/claude-settings-sync"  "$HOME/.local/bin/claude-settings-sync"
link "scripts/tmux-sessions"         "$HOME/.local/bin/tmux-sessions"
link "scripts/tmux-tasks"            "$HOME/.local/bin/tmux-tasks"
link "scripts/tmux-task-run"         "$HOME/.local/bin/tmux-task-run"
link "scripts/tmux-status-left"      "$HOME/.local/bin/tmux-status-left"
link "scripts/tmux-status-right"     "$HOME/.local/bin/tmux-status-right"
link "scripts/tmux-pick"             "$HOME/.local/bin/tmux-pick"

# ─── Phase 6b: Claude Code settings ────────────────────────
# Merge the tracked base with this machine's local overrides. Runs after the
# links above so statusline-custom.sh and hooks/ already resolve.
export DOTFILES
"$DOTFILES/scripts/claude-settings-sync" \
  || warn "claude-settings-sync failed (jq missing?) — ~/.claude/settings.json not regenerated"

# ─── Phase 7: Switch remote to SSH ─────────────────────────
current_remote="$(git -C "$DOTFILES" remote get-url origin 2>/dev/null || true)"
if [[ "$current_remote" == https://* ]]; then
  git -C "$DOTFILES" remote set-url origin git@github.com:GauthierDoppler/dotfiles.git
  echo "switched remote to SSH"
fi

# ─── Phase 8: bun ──────────────────────────────────────────
if ! command -v bun &>/dev/null; then
  echo "Installing bun..."
  curl -fsSL https://bun.sh/install | bash
fi

# ─── Phase 9: Node LTS via fnm ─────────────────────────────
eval "$(fnm env)"
if ! fnm ls 2>/dev/null | grep -q lts-latest; then
  echo "Installing Node LTS via fnm..."
  fnm install --lts
fi
fnm default lts-latest 2>/dev/null || true

# ─── Phase 10: Global packages ─────────────────────────────
if ! command -v claude &>/dev/null; then
  echo "Installing Claude Code CLI..."
  npm install -g @anthropic-ai/claude-code
fi

if ! command -v nvr &>/dev/null; then
  echo "Installing neovim-remote..."
  pipx install neovim-remote
fi

# ─── Phase 11: App registration ────────────────────────────
if [[ -d "$DOTFILES/dot_claude/hooks/ClaudeCodeNotifier.app" ]]; then
  /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$DOTFILES/dot_claude/hooks/ClaudeCodeNotifier.app"
  echo "registered: ClaudeCodeNotifier.app"
fi

echo ""
echo "Done! Open a new terminal or run: exec zsh"
