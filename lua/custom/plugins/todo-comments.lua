return {
  {
    'folke/todo-comments.nvim',
    event = 'VimEnter',
    dependencies = { 'nvim-lua/plenary.nvim' },
    opts = {},
    keys = {
      { '<leader>st', function() Snacks.picker.todo_comments() end, desc = '[S]earch [T]odos' },
    },
  },
}
