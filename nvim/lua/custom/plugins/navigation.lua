return {
  {
    'christoomey/vim-tmux-navigator',
    event = 'VeryLazy',
  },
  {
    's1n7ax/nvim-window-picker',
    version = '2.*',
    opts = {
      hint = 'floating-big-letter',
      selection_chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890',
      filter_rules = {
        autoselect_one = true,
        include_current_win = false,
        bo = {
          filetype = { 'snacks_picker_list', 'snacks_notif', 'notify' },
          buftype = { 'terminal', 'quickfix' },
        },
      },
    },
  },
}
