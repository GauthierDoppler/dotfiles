-- Global autocommands
-- [[ Basic Autocommands ]]
--  See `:help lua-guide-autocommands`

-- Highlight when yanking (copying) text
--  Try it with `yap` in normal mode
--  See `:help vim.hl.on_yank()`
vim.api.nvim_create_autocmd('TextYankPost', {
  desc = 'Highlight when yanking (copying) text',
  group = vim.api.nvim_create_augroup('highlight-yank', { clear = true }),
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
    if gs then pcall(gs.refresh) end
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
    local ok, err = pcall(function()
      vim.api.nvim_buf_call(buf, function() vim.cmd.write { mods = { silent = true } } end)
    end)
    if not ok then vim.notify('auto-save failed: ' .. tostring(err), vim.log.levels.WARN) end
  end,
})

-- Markdown preview: atelier preview
local function md_preview_cursor(buf)
  local url = vim.b[buf].md_preview_cursor
  if not url then return end
  local line = vim.api.nvim_win_get_cursor(0)[1]
  if line == vim.b[buf].md_preview_line then return end
  vim.b[buf].md_preview_line = line
  vim.system { 'curl', '-s', '-m', '1', '-X', 'POST', '-H', 'content-type: application/json', '-d', vim.json.encode { line = line }, url }
end

vim.api.nvim_create_autocmd('FileType', {
  desc = 'Markdown preview in the browser',
  group = vim.api.nvim_create_augroup('md-preview', { clear = true }),
  pattern = 'markdown',
  callback = function(args)
    local buf = args.buf
    vim.keymap.set('n', '<leader>mr', function()
      local file = vim.api.nvim_buf_get_name(buf)
      if file == '' then return vim.notify('md-preview: buffer has no file', vim.log.levels.WARN) end
      local atelier = vim.fn.expand '~/.local/bin/atelier'
      if vim.fn.executable(atelier) == 0 then return vim.notify('md-preview: ' .. atelier .. ' missing -- run ./install.sh', vim.log.levels.ERROR) end
      vim.system({ atelier, 'preview', file }, { text = true }, function(res)
        vim.schedule(function()
          if res.code ~= 0 then return vim.notify(vim.trim(res.stderr or '') ~= '' and vim.trim(res.stderr) or 'md-preview failed', vim.log.levels.ERROR) end
          if not vim.api.nvim_buf_is_valid(buf) then return end
          vim.b[buf].md_preview_cursor = vim.trim(res.stdout):gsub('^(https?://[^/]+)', '%1/__cursor')
          vim.b[buf].md_preview_line = nil
          if vim.api.nvim_get_current_buf() == buf then md_preview_cursor(buf) end
        end)
      end)
    end, { buffer = buf, desc = '[M]arkdown [R]ender in browser' })

    vim.api.nvim_create_autocmd('CursorHold', {
      buffer = buf,
      group = vim.api.nvim_create_augroup('md-preview-' .. buf, { clear = true }),
      callback = function() md_preview_cursor(buf) end,
    })
  end,
})
