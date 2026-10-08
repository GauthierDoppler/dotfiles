#!/bin/bash
# Bootstrap a machine from scratch.
# Run from the dotfiles directory: ./install.sh [--profile desktop|remote]
#
# First run (no SSH key):  generates key, copies pubkey, exits.
# Second run (key exists): installs everything.

set -euo pipefail

DOTFILES="$(cd "$(dirname "$0")" && pwd)"
OS="$(uname -s)"
if [[ "$OS" == Darwin ]]; then PROFILE=desktop; else PROFILE=remote; fi

while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile) PROFILE="${2:?--profile needs desktop or remote}"; shift 2 ;;
    --profile=*) PROFILE="${1#--profile=}"; shift ;;
    *) echo "usage: $0 [--profile desktop|remote]" >&2; exit 2 ;;
  esac
done
case "$PROFILE" in
  desktop|remote) ;;
  *) echo "unknown profile: $PROFILE (desktop or remote)" >&2; exit 2 ;;
esac

warn() { echo "WARNING: $*"; }

# ─── Phase 0: SSH key ──────────────────────────────────────
if [[ ! -f "$HOME/.ssh/github" ]]; then
  echo "No GitHub SSH key found. Setting one up first..."
  echo ""
  "$DOTFILES/scripts/ssh-setup" github
  echo ""
  if [[ "$OS" == Darwin ]]; then
    open "https://github.com/settings/ssh/new"
  fi
  echo "Add the SSH key to GitHub (https://github.com/settings/ssh/new), then run this script again."
  exit 0
fi

# ─── Phase 1: Xcode Command Line Tools ─────────────────────
if [[ "$OS" == Darwin ]] && ! xcode-select -p &>/dev/null; then
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
elif [[ -f /home/linuxbrew/.linuxbrew/bin/brew ]]; then
  eval "$(/home/linuxbrew/.linuxbrew/bin/brew shellenv)"
fi

# ─── Phase 3: Brew Bundle ──────────────────────────────────
echo "Running brew bundle..."
if [[ "$PROFILE" == desktop ]]; then
  brew bundle --file="$DOTFILES/Brewfile"
else
  grep -v '^cask ' "$DOTFILES/Brewfile" | brew bundle --file=-
fi

# ─── Phase 4: Oh My Zsh ────────────────────────────────────
if [[ ! -d "$HOME/.oh-my-zsh" ]]; then
  echo "Installing Oh My Zsh..."
  sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" "" --unattended
fi

# ─── Phase 5: atelier ──────────────────────────────────────
export PATH="$HOME/.cargo/bin:$PATH"
if ! command -v cargo &>/dev/null; then
  echo "Installing Rust via rustup..."
  curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs | sh -s -- -y --no-modify-path
fi
cargo install --locked --path "$DOTFILES/atelier" --root "$HOME/.local" \
  --target-dir "$DOTFILES/atelier/target"

# ─── Phase 6: Links and stubs ──────────────────────────────
"$HOME/.local/bin/atelier" setup --repo "$DOTFILES" --profile "$PROFILE"

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

# Keyboard layout (AZERTY with an unshifted number row — the tmux Prefix + 1..9
# bindings depend on it). Installing it does not SELECT it: that is a one-time
# manual step in System Settings → Keyboard → Input Sources. Writing
# AppleEnabledInputSources / AppleSelectedInputSources with `defaults` is cached
# by the input-source daemon, needs a logout to take, and half-works meanwhile.
if [[ "$PROFILE" == desktop && "$OS" == Darwin ]]; then
  copy_bundle "keyboard/FR-AZERTY-num.bundle" "$HOME/Library/Keyboard Layouts/FR-AZERTY-num.bundle"
fi

# ─── Phase 6b: Claude Code settings ────────────────────────
# Runs after the links so statusline-custom.sh and hooks/ already resolve.
"$HOME/.local/bin/atelier" claude-settings-sync --repo "$DOTFILES" \
  || warn "atelier claude-settings-sync failed — ~/.claude/settings.json not regenerated"

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
"$HOME/.bun/bin/bun" install --cwd "$DOTFILES/scripts/md-preview" --frozen-lockfile

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

# ─── Phase 10b: launchd agents ─────────────────────────────
launch_agent() {
  local label="$1"
  local src="$DOTFILES/launchd/$label.plist"
  local dir="$HOME/Library/LaunchAgents"
  local dest="$dir/$label.plist"
  local domain
  domain="gui/$(id -u)"
  mkdir -p "$dir" 2>/dev/null || true
  if [ ! -w "$dir" ]; then
    warn "$dir is not writable (sudo chown $USER:staff $dir) — $label not installed"
    return 0
  fi
  # A bootstrapped agent can sit at "pended nondemand spawn = speculative" for
  # minutes; kickstart without -k starts it now and leaves a running one alone.
  if cmp -s "$src" "$dest" && launchctl print "$domain/$label" &>/dev/null; then
    launchctl kickstart "$domain/$label" &>/dev/null || true
    echo "agent ok: $label"
    return 0
  fi
  # bootout returns before the service is gone, and a bootstrap issued in that
  # window fails with "5: Input/output error".
  if launchctl bootout "$domain/$label" 2>/dev/null; then
    for _ in $(seq 50); do
      launchctl print "$domain/$label" &>/dev/null || break
      sleep 0.1
    done
  fi
  cp "$src" "$dest"
  if launchctl bootstrap "$domain" "$dest"; then
    launchctl kickstart "$domain/$label" &>/dev/null || true
    echo "loaded: $label"
  else
    warn "launchctl bootstrap failed for $label"
  fi
}

if [[ "$OS" == Darwin ]]; then
  launch_agent "com.theodo.cc-tap.dashboard"
  launch_agent "com.theodo.cc-tap.proxy"
  launch_agent "com.theodo.cc-tap.update"
  launch_agent "com.github.gauthierdoppler.md-preview"
fi

# ─── Phase 11: App registration ────────────────────────────
if [[ "$OS" == Darwin && -d "$DOTFILES/dot_claude/hooks/ClaudeCodeNotifier.app" ]]; then
  /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$DOTFILES/dot_claude/hooks/ClaudeCodeNotifier.app"
  echo "registered: ClaudeCodeNotifier.app"
fi

echo ""
echo "Done! Open a new terminal or run: exec zsh"
