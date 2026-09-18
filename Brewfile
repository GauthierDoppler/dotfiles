# Brewfile — all dependencies, run via: brew bundle --file=~/dotfiles/Brewfile
# Add new tools here, then run ./install.sh or brew bundle

# ─── CLI tools ───────────────────────────────────────────────
brew "neovim"
brew "tmux"
brew "git-delta"
brew "lazygit"
brew "lazydocker"
brew "fnm"
brew "pipx"
brew "terminal-notifier"
brew "go"
brew "pnpm"
brew "gh"
brew "ripgrep"
brew "fd"
brew "fzf"
brew "bat"
brew "eza"
brew "tree"
brew "htop"
brew "yq"
brew "coreutils"
# Lua formatter. CI runs `stylua --check nvim/` on every push touching Lua, so
# it has to be installable locally or the check can only ever fail after a push.
brew "stylua"
# The `tree-sitter` formula is library-only; the binary lives in tree-sitter-cli.
# nvim-treesitter (main) runs `tree-sitter generate` for grammars that ship no
# src/parser.c — kotlin is one, and fails to install without it.
brew "tree-sitter-cli"
brew "jq"
brew "imagemagick"
brew "ghostscript"
brew "poppler"
brew "ruby"
brew "libpq"

# ─── Shell scripting (this repo is bash) ────────────────────
brew "shellcheck"
brew "bats-core"
brew "gitleaks"

# ─── AI tooling ─────────────────────────────────────────────
brew "agent-browser"
# Official Notion CLI; the binary is `ntn`.
cask "notion-cli"

# ─── Desktop apps ───────────────────────────────────────────
cask "ghostty"
cask "raycast"
cask "claude"
