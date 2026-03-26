#!/bin/bash
# Creates symlinks from dotfiles repo to their expected locations
# Run from the dotfiles directory: ./install.sh

DOTFILES="$(cd "$(dirname "$0")" && pwd)"

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
link "lazydocker"           "$HOME/.config/lazydocker"
link "lazygit"             "$HOME/.config/lazygit"
link "nvim"                "$HOME/.config/nvim"
link "zed"                 "$HOME/.config/zed"

# ~/.config/* individual files
link "git/ignore"          "$HOME/.config/git/ignore"

# ~/ dotfiles (dot_ prefix becomes .)
link "dot_gitconfig"       "$HOME/.gitconfig"
link "dot_tmux.conf"       "$HOME/.tmux.conf"
link "dot_zshrc"           "$HOME/.zshrc"

# ~/.claude/* individual files
link "dot_claude/settings.json"        "$HOME/.claude/settings.json"
link "dot_claude/CLAUDE.md"            "$HOME/.claude/CLAUDE.md"
link "dot_claude/statusline-custom.sh" "$HOME/.claude/statusline-custom.sh"
link "dot_claude/hooks"                "$HOME/.claude/hooks"


# --- Dependencies ---

ensure() {
  local cmd="$1"
  shift
  if command -v "$cmd" &>/dev/null; then
    echo "found: $cmd"
  else
    echo "installing: $cmd"
    "$@"
  fi
}

ensure "pipx"                brew install pipx
ensure "nvr"                 pipx install neovim-remote
ensure "terminal-notifier"   brew install terminal-notifier

# Register notification app bundle for Claude Code icon
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$DOTFILES/dot_claude/hooks/ClaudeCodeNotifier.app"
echo "registered: ClaudeCodeNotifier.app"
