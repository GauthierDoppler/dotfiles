return {
  {
    'akinsho/bufferline.nvim',
    version = '*',
    event = 'VimEnter',
    dependencies = { 'nvim-tree/nvim-web-devicons' },
    opts = {
      options = {
        diagnostics = 'nvim_lsp',
        offsets = {
          { filetype = 'snacks_layout_box', text = 'File Explorer', highlight = 'Directory', separator = true },
        },
      },
    },
  },
  {
    'rachartier/tiny-inline-diagnostic.nvim',
    event = 'VeryLazy',
    priority = 1000,
    opts = {},
  },
  {
    'folke/trouble.nvim',
    cmd = 'Trouble',
    keys = {
      { '<leader>qq', '<cmd>Trouble diagnostics toggle<cr>', desc = '[Q]uickfix diagnostics' },
      { '<leader>qb', '<cmd>Trouble diagnostics toggle filter.buf=0<cr>', desc = '[Q]uickfix [B]uffer diagnostics' },
      { '<leader>ql', '<cmd>Trouble loclist toggle<cr>', desc = '[Q]uickfix [L]ocation list' },
      { '<leader>qf', '<cmd>Trouble qflist toggle<cr>', desc = '[Q]uickfix [F]ix list' },
    },
    opts = {
      focus = true,
      keys = {
        ['<cr>'] = 'jump_close',
      },
      win = {
        type = 'float',
        border = 'rounded',
        position = { 0.5, 0.5 },
        size = { width = 0.8, height = 0.6 },
        wo = { wrap = true },
      },
    },
  },
  {
    'OXY2DEV/markview.nvim',
    lazy = false,
    ft = 'markdown',
    keys = {
      { '<leader>mr', '<cmd>Markview toggle<cr>', desc = '[M]arkdown [R]ender toggle' },
    },
  },
}
