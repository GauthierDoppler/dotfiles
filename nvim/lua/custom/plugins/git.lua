vim.keymap.set('n', '<leader>kg', function() Snacks.lazygit() end, { desc = 'Lazy[G]it (repo status)' })
vim.keymap.set({ 'n', 'v' }, '<leader>go', function() Snacks.gitbrowse() end, { desc = '[G]it [O]pen in browser' })

vim.api.nvim_create_user_command('LazygitFocusEditor', function()
  if vim.api.nvim_win_get_config(0).relative ~= '' then vim.cmd.close() end
  for _, win in ipairs(vim.api.nvim_tabpage_list_wins(0)) do
    local buf = vim.api.nvim_win_get_buf(win)
    if vim.api.nvim_win_get_config(win).relative == '' and vim.bo[buf].buftype == '' and not vim.bo[buf].filetype:match '^snacks' then
      vim.api.nvim_set_current_win(win)
      return
    end
  end
end, { desc = 'Close the lazygit float and focus an editor window' })

return {}
