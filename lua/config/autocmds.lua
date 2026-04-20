-- Global autocommands
-- [[ Basic Autocommands ]]
--  See `:help lua-guide-autocommands`

-- Highlight when yanking (copying) text
--  Try it with `yap` in normal mode
--  See `:help vim.hl.on_yank()`
vim.api.nvim_create_autocmd('TextYankPost', {
  desc = 'Highlight when yanking (copying) text',
  group = vim.api.nvim_create_augroup('kickstart-highlight-yank', { clear = true }),
  callback = function() vim.hl.on_yank() end,
})

-- Folding setup for selected languages
vim.api.nvim_create_autocmd('FileType', {
  desc = 'Enable treesitter folding for Go, TS, Python, and Ruby',
  group = vim.api.nvim_create_augroup('classic_indent_folds', { clear = true }),
  pattern = {
    'go',
    'typescript',
    'typescriptreact',
    'javascript',
    'javascriptreact',
    'python',
    'ruby',
  },
  callback = function()
    vim.opt_local.foldmethod = 'expr'
    vim.opt_local.foldexpr = 'v:lua.vim.treesitter.foldexpr()'
    vim.opt_local.foldenable = true
    vim.opt_local.foldlevel = 99
    vim.opt_local.foldlevelstart = 99
    vim.opt_local.foldminlines = 1
  end,
})

-- Trigger autoread and refresh gitsigns when Neovim regains focus or a buffer is entered
vim.api.nvim_create_autocmd({ 'FocusGained', 'BufEnter', 'TermClose' }, {
  desc = 'Check for external file changes and refresh gitsigns',
  group = vim.api.nvim_create_augroup('auto-reload-files', { clear = true }),
  callback = function()
    vim.cmd 'checktime'
    local gs = package.loaded['gitsigns']
    if gs then
      pcall(gs.refresh)
    end
  end,
})

-- Auto-save the current buffer on focus lost, buffer leave, or leaving insert mode.
-- Scoped to the triggering buffer (not all buffers) and writes are pcalled so failures
-- surface in :messages instead of being swallowed by `silent!`.
vim.api.nvim_create_autocmd({ 'FocusLost', 'BufLeave', 'InsertLeave' }, {
  desc = 'Auto-save current buffer',
  group = vim.api.nvim_create_augroup('auto-save', { clear = true }),
  callback = function(args)
    local buf = args.buf
    if not vim.bo[buf].modified or not vim.bo[buf].modifiable then return end
    if vim.bo[buf].buftype ~= '' or vim.api.nvim_buf_get_name(buf) == '' then return end
    local ok, err = pcall(function() vim.api.nvim_buf_call(buf, function() vim.cmd.write { mods = { silent = true } } end) end)
    if not ok then vim.notify('auto-save failed: ' .. tostring(err), vim.log.levels.WARN) end
  end,
})
