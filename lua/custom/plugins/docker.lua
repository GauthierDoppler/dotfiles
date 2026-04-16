vim.keymap.set('n', '<leader>gd', function()
  Snacks.terminal('lazydocker', { win = { width = 0.9, height = 0.9 } })
end, { desc = 'LazyDocker' })

return {}
