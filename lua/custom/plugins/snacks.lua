return {
  'folke/snacks.nvim',
  lazy = false,
  priority = 1000,
  opts = {
    indent = { enabled = true },
    input = { enabled = true },
    notifier = { enabled = true },
    lazygit = { enabled = true },
    picker = {
      enabled = true,
      matcher = { smartcase = true },
      actions = {
        glob_filter = function(picker)
          local text = picker.input:get()
          picker.input:set(nil, text .. ' -- -g **/*.')
        end,
        focus_editor = function(picker)
          vim.api.nvim_set_current_win(picker.main)
        end,
        explorer_search_open = function(picker)
          local item = picker:current()
          if not item then return end
          local searching = picker.input.filter.meta.searching
          if searching and not item.dir then
            -- Searching: open file directly in editor
            Snacks.picker.actions.jump(picker, item, {})
          elseif searching and item.dir then
            require('snacks.explorer.actions').update(picker, { target = item.file })
          elseif item.dir then
            -- Not searching: toggle directory
            require('snacks.explorer.tree'):toggle(item.file)
            require('snacks.explorer.actions').update(picker, { refresh = true })
          else
            -- Not searching: open file
            Snacks.picker.actions.jump(picker, item, {})
          end
        end,
      },
      sources = {
        files = { hidden = true },
        explorer = {
          hidden = true,
          ignored = true,
          exclude = { 'node_modules', '.git', '.DS_Store', '__pycache__', '.cache' },
          matcher = { sort_empty = false, fuzzy = true, smartcase = true },
          win = {
            list = {
              keys = {
                ['<CR>'] = { 'explorer_search_open', desc = 'Open file / navigate dir' },
                ['l'] = { 'explorer_search_open', desc = 'Open file / navigate dir' },
                ['<leader>ww'] = { 'focus_editor', desc = 'Switch to editor' },
                ['<C-w>w'] = { 'focus_editor', desc = 'Switch to editor' },
              },
            },
            input = {
              keys = {
                ['<CR>'] = { 'explorer_search_open', mode = { 'n', 'i' }, desc = 'Open file from search' },
                ['<leader>ww'] = { 'focus_editor', mode = { 'n', 'i' }, desc = 'Switch to editor' },
                ['<C-w>w'] = { 'focus_editor', mode = { 'n', 'i' }, desc = 'Switch to editor' },
              },
            },
          },
        },
        grep = {
          hidden = true,
          win = {
            input = {
              keys = {
                ['<C-f>'] = { 'glob_filter', mode = { 'i', 'n' }, desc = 'Add file glob filter' },
              },
            },
          },
        },
      },
    },
    explorer = {
      enabled = true,
      replace_netrw = true,
    },
    bufdelete = { enabled = true },
    rename = { enabled = true },
    bigfile = { enabled = true },
    quickfile = { enabled = true },
    gitbrowse = { enabled = true },
    image = { enabled = true },
    statuscolumn = { enabled = true },
    scroll = {
      enabled = true,
      animate = {
        duration = { step = 10, total = 100 },
        easing = 'linear',
      },
      animate_repeat = {
        delay = 50,
        duration = { step = 5, total = 25 },
        easing = 'linear',
      },
    },
    words = { enabled = true },
    terminal = { enabled = true },
  },
}
