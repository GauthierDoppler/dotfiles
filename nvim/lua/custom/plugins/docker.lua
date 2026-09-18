vim.keymap.set('n', '<leader>kd', function() Snacks.terminal('lazydocker', { win = { width = 0.9, height = 0.9 } }) end, { desc = 'Lazy[D]ocker' })

return {}
