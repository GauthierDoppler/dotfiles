local function assertf(cond, msg)
  if not cond then
    error(msg)
  end
end

-- Navigation specs: vim-tmux-navigator + nvim-window-picker
local nav_specs = require('custom.plugins.navigation')
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
assertf(vim.fn.maparg('<leader>gg', 'n') ~= '', '<leader>gg (lazygit) mapping is missing')

print('smoke:ok')
