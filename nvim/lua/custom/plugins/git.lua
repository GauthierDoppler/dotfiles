vim.keymap.set('n', '<leader>kg', function() Snacks.lazygit() end, { desc = 'Lazy[G]it (repo status)' })
vim.keymap.set({ 'n', 'v' }, '<leader>go', function() Snacks.gitbrowse() end, { desc = '[G]it [O]pen in browser' })

return {}
