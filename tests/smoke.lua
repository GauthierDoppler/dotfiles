local function assertf(cond, msg)
  if not cond then
    error(msg)
  end
end

local nav_specs = require('custom.plugins.navigation')
assertf(type(nav_specs) == 'table', 'navigation specs missing')

local neo_override
for _, spec in ipairs(nav_specs) do
  if spec[1] == 'nvim-neo-tree/neo-tree.nvim' then
    neo_override = spec
    break
  end
end

assertf(neo_override ~= nil, 'neo-tree override spec missing')
assertf(type(neo_override.opts) == 'function', 'neo-tree override opts must be a function')

local neo_opts = {}
neo_override.opts(nil, neo_opts)
assertf(neo_opts.filesystem ~= nil, 'neo-tree filesystem override missing')
assertf(neo_opts.filesystem.use_libuv_file_watcher == true, 'neo-tree file watcher override is not enabled')

assertf(vim.fn.exists(':Neotree') == 2, ':Neotree command is missing')
assertf(vim.fn.exists(':LazyGit') == 2, ':LazyGit command is missing')
assertf(vim.fn.maparg('<leader>gg', 'n') ~= '', '<leader>gg mapping is missing')

local ok_picker = pcall(require, 'window-picker')
assertf(ok_picker, 'window-picker module cannot be required')

print('smoke:ok')
