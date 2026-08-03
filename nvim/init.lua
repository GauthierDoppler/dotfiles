-- Entry point for this Neovim config.

-- Set <space> as the leader key (must happen before plugin loading)
vim.g.mapleader = ' '
vim.g.maplocalleader = ' '

-- Set to true if you have a Nerd Font installed and selected in the terminal.
-- ghostty/config pins font-family to "JetBrainsMono Nerd Font Mono"; this was
-- false back when font-family was unset and ghostty fell back to a font with no
-- Nerd Font coverage, where every glyph rendered as a blank cell.
vim.g.have_nerd_font = true

-- No UI attached — this is `nvim --headless`, i.e. scripts/test-config.sh.
-- Guards the install-on-startup paths (treesitter parsers, mason tools): the
-- test harness wipes XDG on every run, so those would re-download everything
-- and then get killed mid-flight by `+qa`. Must be set before config.lazy.
vim.g.headless = #vim.api.nvim_list_uis() == 0

require 'config.options'
require 'config.keymaps'
require 'config.autocmds'
require 'config.lazy'

-- vim: ts=2 sts=2 sw=2 et
