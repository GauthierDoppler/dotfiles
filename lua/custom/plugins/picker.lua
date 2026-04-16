-- Picker keymaps (snacks.picker)
-- LSP navigation keymaps are set in the LspAttach autocommand below.

-- File explorer
vim.keymap.set('n', '<leader>e', function() Snacks.explorer() end, { desc = 'File [E]xplorer toggl[e]' })
vim.keymap.set('n', '\\', function() Snacks.explorer() end, { desc = 'File Explorer' })

-- File and grep search
vim.keymap.set('n', '<leader>sf', function() Snacks.picker.files() end, { desc = '[S]earch [F]iles' })
vim.keymap.set('n', '<leader>sn', function() Snacks.picker.files { cwd = vim.fn.stdpath 'config' } end, { desc = '[S]earch [N]eovim files' })
vim.keymap.set('n', '<leader>sg', function() Snacks.picker.grep() end, { desc = '[S]earch by [G]rep' })
vim.keymap.set({ 'n', 'v' }, '<leader>sw', function() Snacks.picker.grep_word() end, { desc = '[S]earch current [W]ord' })

-- General search
vim.keymap.set('n', '<leader>sh', function() Snacks.picker.help() end, { desc = '[S]earch [H]elp' })
vim.keymap.set('n', '<leader>sk', function() Snacks.picker.keymaps() end, { desc = '[S]earch [K]eymaps' })
vim.keymap.set('n', '<leader>ss', function() Snacks.picker.pickers() end, { desc = '[S]earch [S]elect Picker' })
vim.keymap.set('n', '<leader>sd', function() Snacks.picker.diagnostics() end, { desc = '[S]earch [D]iagnostics' })
vim.keymap.set('n', '<leader>sr', function() Snacks.picker.resume() end, { desc = '[S]earch [R]esume' })
vim.keymap.set('n', '<leader>s.', function() Snacks.picker.recent() end, { desc = '[S]earch Recent Files ("." for repeat)' })
vim.keymap.set('n', '<leader>sc', function() Snacks.picker.commands() end, { desc = '[S]earch [C]ommands' })
vim.keymap.set('n', '<leader><leader>', function() Snacks.picker.buffers() end, { desc = '[ ] Find existing buffers' })
vim.keymap.set('n', '<leader>cr', function() Snacks.picker.registers() end, { desc = '[C]lipboard [R]egisters (search & paste)' })

vim.keymap.set('n', '<leader>/', function() Snacks.picker.lines() end, { desc = '[/] Fuzzily search in current buffer' })

vim.keymap.set(
  'n',
  '<leader>s/',
  function() Snacks.picker.grep { buffers = true } end,
  { desc = '[S]earch [/] in Open Files' }
)

-- LSP navigation keymaps (set per buffer on LspAttach)
vim.api.nvim_create_autocmd('LspAttach', {
  group = vim.api.nvim_create_augroup('snacks-picker-lsp-attach', { clear = true }),
  callback = function(event)
    local buf = event.buf

    vim.keymap.set('n', 'grr', function() Snacks.picker.lsp_references() end, { buffer = buf, desc = '[G]oto [R]eferences' })
    vim.keymap.set('n', 'gri', function() Snacks.picker.lsp_implementations() end, { buffer = buf, desc = '[G]oto [I]mplementation' })
    vim.keymap.set('n', 'grd', function() Snacks.picker.lsp_definitions() end, { buffer = buf, desc = '[G]oto [D]efinition' })
    vim.keymap.set('n', 'gO', function() Snacks.picker.lsp_symbols() end, { buffer = buf, desc = 'Open Document Symbols' })
    vim.keymap.set('n', 'gW', function() Snacks.picker.lsp_workspace_symbols() end, { buffer = buf, desc = 'Open Workspace Symbols' })
    vim.keymap.set('n', 'grt', function() Snacks.picker.lsp_type_definitions() end, { buffer = buf, desc = '[G]oto [T]ype Definition' })

    vim.keymap.set('n', 'grn', vim.lsp.buf.rename, { buffer = buf, desc = '[R]e[n]ame' })
    vim.keymap.set({ 'n', 'x' }, 'gra', vim.lsp.buf.code_action, { buffer = buf, desc = 'Code [A]ction' })

    -- Go to source definition (skips barrel files in TS), falls back to declaration
    vim.keymap.set('n', 'grD', function()
      local clients = vim.lsp.get_clients { bufnr = buf, name = 'vtsls' }
      if #clients > 0 then
        clients[1]:exec_cmd {
          command = 'typescript.goToSourceDefinition',
          arguments = { vim.uri_from_bufnr(buf), vim.lsp.util.make_position_params(0, 'utf-16').position },
        }
      else
        vim.lsp.buf.declaration()
      end
    end, { buffer = buf, desc = '[G]oto Source [D]efinition' })
  end,
})

return {}
