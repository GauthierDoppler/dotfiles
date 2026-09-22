-- Global keymaps and diagnostics
-- [[ Basic Keymaps ]]
--  See `:help vim.keymap.set()`

-- Clear highlights on search when pressing <Esc> in normal mode
--  See `:help hlsearch`
vim.keymap.set('n', '<Esc>', '<cmd>nohlsearch<CR>')

-- Diagnostic Config & Keymaps
-- See :help vim.diagnostic.Opts
vim.diagnostic.config {
  update_in_insert = false,
  severity_sort = true,
  float = { border = 'rounded', source = 'if_many' },
  underline = { severity = vim.diagnostic.severity.ERROR },

  -- Disabled: tiny-inline-diagnostic handles virtual text rendering
  virtual_text = false,
  virtual_lines = false,

  -- Auto open the float, so you can easily read the errors when jumping with `[d` and `]d`
  jump = { float = true },
}

-- <leader>q is mapped by trouble.nvim in custom/plugins/ui.lua

-- Neovim 0.12 ships global LSP mappings (grn gra grr gri grt grx gO) whose `desc`
-- is the literal call text -- which-key renders them as "vim.lsp.buf.rename()".
-- custom/plugins/picker.lua overrides most of them buffer-locally on LspAttach with
-- snacks pickers, but these globals are what you see in any buffer with no LSP
-- attached, and `grx` is never overridden at all. Relabel them in place.
local lsp_defaults = {
  { 'n', 'grn', vim.lsp.buf.rename, '[R]e[n]ame symbol' },
  { { 'n', 'x' }, 'gra', vim.lsp.buf.code_action, 'Code [A]ction' },
  { 'n', 'grr', vim.lsp.buf.references, '[G]oto [R]eferences' },
  { 'n', 'gri', vim.lsp.buf.implementation, '[G]oto [I]mplementation' },
  { 'n', 'grt', vim.lsp.buf.type_definition, '[G]oto [T]ype definition' },
  { 'n', 'grx', vim.lsp.codelens.run, 'Run code lens' },
  { 'n', 'gO', vim.lsp.buf.document_symbol, 'Document symbols ([O]utline)' },
}
for _, m in ipairs(lsp_defaults) do
  vim.keymap.set(m[1], m[2], m[3], { desc = m[4] })
end

-- Exit terminal mode in the builtin terminal with a shortcut that is a bit easier
-- for people to discover. Otherwise, you normally need to press <C-\><C-n>, which
-- is not what someone will guess without a bit more experience.
--
-- NOTE: This won't work in all terminal emulators/tmux/etc. Try your own mapping
-- or just use <C-\><C-n> to exit terminal mode
vim.keymap.set('t', '<Esc><Esc>', '<C-\\><C-n>', { desc = 'Exit terminal mode' })

-- TIP: Disable arrow keys in normal mode
-- vim.keymap.set('n', '<left>', '<cmd>echo "Use h to move!!"<CR>')
-- vim.keymap.set('n', '<right>', '<cmd>echo "Use l to move!!"<CR>')
-- vim.keymap.set('n', '<up>', '<cmd>echo "Use k to move!!"<CR>')
-- vim.keymap.set('n', '<down>', '<cmd>echo "Use j to move!!"<CR>')

-- Keybinds to make split navigation easier.
--  Use CTRL+<hjkl> to switch between windows
--
--  See `:help wincmd` for a list of all window commands
vim.keymap.set('n', '<C-h>', '<C-w><C-h>', { desc = 'Move focus to the left window' })
vim.keymap.set('n', '<C-l>', '<C-w><C-l>', { desc = 'Move focus to the right window' })
vim.keymap.set('n', '<C-j>', '<C-w><C-j>', { desc = 'Move focus to the lower window' })
vim.keymap.set('n', '<C-k>', '<C-w><C-k>', { desc = 'Move focus to the upper window' })

-- NOTE: Some terminals have colliding keymaps or are not able to send distinct keycodes
-- vim.keymap.set("n", "<C-S-h>", "<C-w>H", { desc = "Move window to the left" })
-- vim.keymap.set("n", "<C-S-l>", "<C-w>L", { desc = "Move window to the right" })
-- vim.keymap.set("n", "<C-S-j>", "<C-w>J", { desc = "Move window to the lower" })
-- vim.keymap.set("n", "<C-S-k>", "<C-w>K", { desc = "Move window to the upper" })

-- Window management
vim.keymap.set('n', '<leader>wh', '<C-w>h', { desc = '[W]indow focus left' })
vim.keymap.set('n', '<leader>wj', '<C-w>j', { desc = '[W]indow focus down' })
vim.keymap.set('n', '<leader>wk', '<C-w>k', { desc = '[W]indow focus up' })
vim.keymap.set('n', '<leader>wl', '<C-w>l', { desc = '[W]indow focus right' })
vim.keymap.set('n', '<leader>ws', '<cmd>split<CR>', { desc = '[W]indow [S]plit horizontal' })
vim.keymap.set('n', '<leader>wv', '<cmd>vsplit<CR>', { desc = '[W]indow [V]split vertical' })
vim.keymap.set('n', '<leader>wq', '<cmd>close<CR>', { desc = '[W]indow [Q]uit' })
vim.keymap.set('n', '<leader>ww', '<C-w>w', { desc = '[W]indow cycle next' })

-- Buffer management
vim.keymap.set('n', '<leader>bq', function() Snacks.bufdelete() end, { desc = '[B]uffer [Q]uit' })
vim.keymap.set('n', '<leader>bn', '<cmd>bnext<CR>', { desc = '[B]uffer [N]ext' })
vim.keymap.set('n', '<leader>bp', '<cmd>bprev<CR>', { desc = '[B]uffer [P]revious' })
vim.keymap.set('n', '<leader>bQ', function() Snacks.bufdelete.all() end, { desc = '[B]uffer [Q]uit all' })
vim.keymap.set('n', '<leader>bo', function() Snacks.bufdelete.other() end, { desc = '[B]uffer close [O]thers' })

-- Copy file path
vim.keymap.set('n', '<leader>cp', function() vim.fn.setreg('+', vim.fn.expand '%:~:.') end, { desc = '[C]lipboard relative [P]ath' })
vim.keymap.set('n', '<leader>cP', function() vim.fn.setreg('+', vim.fn.expand '%:p') end, { desc = '[C]lipboard full [P]ath' })

-- Markdown preview: dotfiles/scripts/md-preview, watching the file on disk.
vim.keymap.set('n', '<leader>mr', function()
  local file = vim.fn.expand '%:p'
  if file == '' then return vim.notify('md-preview: buffer has no file', vim.log.levels.WARN) end
  if vim.fn.executable 'md-preview' == 0 then return vim.notify('md-preview: not on PATH -- run ./install.sh', vim.log.levels.ERROR) end
  vim.system({ 'md-preview', file }, { text = true, detach = true }, function(res)
    if res.code ~= 0 then
      vim.schedule(function() vim.notify(vim.trim(res.stderr or '') ~= '' and res.stderr or 'md-preview failed', vim.log.levels.ERROR) end)
    end
  end)
end, { desc = '[M]arkdown [R]ender in browser' })

-- IDE Cheatsheet
vim.keymap.set('n', '<leader>?', function()
  local lines = {
    '',
    '   Code ─────────────────────────────',
    '   grd            Goto definition',
    '   grD            Goto source definition',
    '   grr            Goto references',
    '   gri            Goto implementation',
    '   grt            Goto type definition',
    '   grn            Rename symbol',
    '   gra            Code action',
    '   grx            Run code lens',
    '   gO / gW        Document / workspace symbols',
    '   <leader>f      Format buffer',
    '   <leader>th     Toggle inlay hints',
    '   [d  ]d         Prev / next diagnostic',
    '',
    '   Search  <leader>s ───────────────',
    '   sf sg sw       Files / grep / word',
    '   sd sk sh       Diagnostics / keymaps / help',
    '   sr s.          Resume / recent',
    '   st             Todos',
    '   <leader><leader>  Buffers',
    '   <leader>/      Lines in buffer',
    '',
    '   Git  <leader>g ──────────────────',
    '   gs / gS        Stage hunk / buffer',
    '   gr / gR        Reset hunk / buffer',
    '   gu             Undo stage hunk',
    '   gp             Preview hunk inline',
    '   gb             Blame line',
    '   gD             Diff against index',
    '   go             Open in browser',
    '   [c  ]c         Prev / next hunk',
    '',
    '   Debug  <leader>d ─────────────────',
    '   dc             Continue / start',
    '   di do dO       Step into / over / out',
    '   db / dB        Breakpoint / conditional',
    '   dt du          Terminate / toggle UI',
    '   dr dl          Toggle REPL / run last',
    '',
    '   Windows & buffers ────────────────',
    '   <leader>w hjkl Focus window',
    '   <leader>w svq  Split / vsplit / close',
    '   <leader>b nqpo Buffer next/quit/prev/others',
    '',
    '   Tools & UI ──────────────────────',
    '   <leader>kg kd  LazyGit / LazyDocker',
    '   <leader>e  \\   File explorer',
    '   <leader>q qbl  Trouble diagnostics / buf / loc',
    '   <leader>t bd   Toggle git blame / deleted',
    '   <leader>c pPr  Copy path / full / registers',
    '   <leader>mr     Markdown preview in browser',
    '',
    '   q / <Esc> to close',
    '',
  }

  local buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
  vim.bo[buf].modifiable = false
  vim.bo[buf].bufhidden = 'wipe'

  local width = 48
  -- Clamp so the float still opens on a short terminal (the list is ~57 lines);
  -- the buffer scrolls for the overflow.
  local height = math.min(#lines, vim.o.lines - 4)
  local win = vim.api.nvim_open_win(buf, true, {
    relative = 'editor',
    width = width,
    height = height,
    col = math.floor((vim.o.columns - width) / 2),
    row = math.floor((vim.o.lines - height) / 2),
    style = 'minimal',
    border = 'rounded',
    title = ' Cheatsheet ',
    title_pos = 'center',
  })

  local close = function() vim.api.nvim_win_close(win, true) end
  vim.keymap.set('n', 'q', close, { buffer = buf })
  vim.keymap.set('n', '<Esc>', close, { buffer = buf })
end, { desc = 'IDE [?] Cheatsheet' })
