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
