local function assertf(cond, msg)
  if not cond then error(msg) end
end

-- Navigation specs: vim-tmux-navigator + nvim-window-picker
local nav_specs = require 'custom.plugins.navigation'
assertf(type(nav_specs) == 'table', 'navigation specs missing')

local has_window_picker = false
for _, spec in ipairs(nav_specs) do
  if spec[1] == 's1n7ax/nvim-window-picker' then
    has_window_picker = true
    break
  end
end
assertf(has_window_picker, 'nvim-window-picker spec missing from navigation')

local ok_picker = pcall(require, 'window-picker')
assertf(ok_picker, 'window-picker module cannot be required')

-- Snacks (file explorer + picker + lazygit live here).
-- Snacks.explorer / Snacks.lazygit are callable tables (have __call), not raw functions.
local function callable(x)
  if type(x) == 'function' then return true end
  if type(x) ~= 'table' then return false end
  local mt = getmetatable(x)
  return mt ~= nil and type(mt.__call) == 'function'
end

assertf(type(_G.Snacks) == 'table', 'Snacks global not loaded')
assertf(callable(Snacks.explorer), 'Snacks.explorer not callable')
assertf(type(Snacks.picker) == 'table', 'Snacks.picker missing')
assertf(callable(Snacks.picker.files), 'Snacks.picker.files not callable')
assertf(callable(Snacks.lazygit), 'Snacks.lazygit not callable')

-- Core keymaps that must be wired for daily use
assertf(vim.fn.maparg('<leader>e', 'n') ~= '', '<leader>e (explorer) mapping is missing')
assertf(vim.fn.maparg('<leader>sf', 'n') ~= '', '<leader>sf (find files) mapping is missing')
assertf(vim.fn.maparg('<leader>kg', 'n') ~= '', '<leader>kg (lazygit) mapping is missing')
assertf(vim.fn.maparg('<leader>kd', 'n') ~= '', '<leader>kd (lazydocker) mapping is missing')

-- Regression guards for the keymap cleaning pass.

-- Neovim's global LSP defaults ship `desc` values that are the literal call text
-- ("vim.lsp.buf.rename()"); config/keymaps.lua relabels them.
for _, lhs in ipairs { 'grn', 'gra', 'grr', 'gri', 'grt', 'grx', 'gO' } do
  local m = vim.fn.maparg(lhs, 'n', false, true)
  assertf(m and m.desc, lhs .. ' has no desc')
  assertf(not m.desc:match 'vim%.lsp', lhs .. ' still has the raw default desc: ' .. m.desc)
end

-- Every <leader> mapping must carry a desc, or it shows up bare in which-key.
for _, m in ipairs(vim.api.nvim_get_keymap 'n') do
  if m.lhs:match '^ ' or m.lhs:match '^<Space>' then
    assertf(m.desc and m.desc ~= '', 'leader mapping without desc: ' .. m.lhs)
  end
end

-- ktlint 1.8 writes its INFO banner to stderr ahead of the JSON report, which
-- makes nvim-lint's vim.json.decode throw and silences every diagnostic.
local ktlint_args = require('lint').linters.ktlint.args
assertf(vim.tbl_contains(ktlint_args, '--log-level=none'), 'ktlint linter is missing --log-level=none')

print 'smoke:ok'
