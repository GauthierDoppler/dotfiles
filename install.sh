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

# ─── Phase 5: Git submodules ───────────────────────────────
echo "Initializing submodules..."
git -C "$DOTFILES" submodule update --init --recursive || warn "submodule init failed (SSH key not on GitHub?)"

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
link "dot_gitconfig"       "$HOME/.gitconfig"
link "dot_tmux.conf"       "$HOME/.tmux.conf"
link "dot_zshrc"           "$HOME/.zshrc"

# ~/.claude/*
link "dot_claude/settings.json"        "$HOME/.claude/settings.json"
link "dot_claude/CLAUDE.md"            "$HOME/.claude/CLAUDE.md"
link "dot_claude/statusline-custom.sh" "$HOME/.claude/statusline-custom.sh"
link "dot_claude/hooks"                "$HOME/.claude/hooks"

# Scripts
mkdir -p "$HOME/.local/bin"
link "scripts/ssh-setup"   "$HOME/.local/bin/ssh-setup"

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

# ─── Phase 12: .zshrc.local ────────────────────────────────
if [[ ! -f "$HOME/.zshrc.local" ]]; then
  cat > "$HOME/.zshrc.local" << 'EOF'
# Machine-specific config — not tracked in dotfiles.
# Add local PATH additions, project-specific tools, etc. here.
EOF
  echo "created: ~/.zshrc.local"
fi

echo ""
echo "Done! Open a new terminal or run: exec zsh"
