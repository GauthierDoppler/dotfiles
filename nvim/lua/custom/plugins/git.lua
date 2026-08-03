vim.keymap.set('n', '<leader>gg', function() Snacks.lazygit() end, { desc = 'LazyGit (Repo Status)' })
vim.keymap.set({ 'n', 'v' }, '<leader>go', function() Snacks.gitbrowse() end, { desc = '[G]it [O]pen in browser' })

return {}
