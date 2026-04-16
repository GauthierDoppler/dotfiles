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

-- Auto-save all modified buffers on focus lost, buffer leave, or leaving insert mode
vim.api.nvim_create_autocmd({ 'FocusLost', 'BufLeave', 'InsertLeave', 'TextChanged' }, {
  desc = 'Auto-save all modified buffers',
  group = vim.api.nvim_create_augroup('auto-save', { clear = true }),
  callback = function()
    for _, buf in ipairs(vim.api.nvim_list_bufs()) do
      if vim.bo[buf].modified and vim.bo[buf].modifiable and vim.bo[buf].buftype == '' and vim.api.nvim_buf_get_name(buf) ~= '' then
        vim.api.nvim_buf_call(buf, function() vim.cmd 'silent! w' end)
      end
    end
  end,
})

-- Force-close buffers without prompting (auto-save handles persistence)
vim.api.nvim_create_autocmd('BufDelete', {
  desc = 'Force-delete modified buffers since auto-save handles persistence',
  group = vim.api.nvim_create_augroup('force-buf-delete', { clear = true }),
  callback = function(args)
    if vim.bo[args.buf].modified then
      vim.bo[args.buf].modified = false
    end
  end,
})
