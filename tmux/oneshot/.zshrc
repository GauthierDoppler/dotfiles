# ZDOTDIR for the tmux Prefix + Enter popup: an ordinary interactive zsh that
# takes one command, reports how it went, and closes on any key.
#
# With ZDOTDIR set, zsh reads this file INSTEAD of ~/.zshrc, so the real one is
# sourced explicitly. That indirection is the price of the hooks below: zsh has
# no --rcfile flag, and only an rc file can install a precmd hook.

# Both of these resolve against ${ZDOTDIR:-$HOME} — macOS's /etc/zshrc sets
# HISTFILE that way and oh-my-zsh derives ZSH_COMPDUMP the same — so left alone
# zsh writes them into this directory, which is a symlink into the dotfiles repo.
# Pinning them to $HOME also shares the popup's history with every other shell.
#
# ZSH_COMPDUMP has to be set BEFORE oh-my-zsh loads, which only fills a default.
export ZSH_COMPDUMP="$HOME/.zcompdump-${HOST%%.*}-${ZSH_VERSION}"

source "$HOME/.zshrc"

# AFTER the source: /etc/zshrc assigns HISTFILE unconditionally, so an earlier
# assignment here would simply be overwritten.
export HISTFILE="$HOME/.zsh_history"

# preexec fires only when a command is really about to run, so a bare Enter at
# the prompt does not count as the one shot being spent.
_oneshot_ran=0
_oneshot_preexec() { _oneshot_ran=1 }

_oneshot_precmd() {
  # Must be the first line: anything before it overwrites $? .
  local st=$?
  (( _oneshot_ran )) || return
  if (( st == 0 )); then
    print -Pn "%F{green}✓%f  ·  any key to close"
  else
    print -Pn "%F{red}✗ exit $st%f  ·  any key to close"
  fi
  read -k1
  exit $st
}

# add-zsh-hook appends, so oh-my-zsh's own hooks (already registered by the
# source above) keep running alongside these.
autoload -Uz add-zsh-hook
add-zsh-hook preexec _oneshot_preexec
add-zsh-hook precmd  _oneshot_precmd
