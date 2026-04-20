-- [[ Install `lazy.nvim` plugin manager ]]
--    See `:help lazy.nvim.txt` or https://github.com/folke/lazy.nvim for more info
local lazypath = vim.fn.stdpath 'data' .. '/lazy/lazy.nvim'
if not (vim.uv or vim.loop).fs_stat(lazypath) then
  local lazyrepo = 'https://github.com/folke/lazy.nvim.git'
  local out = vim.fn.system { 'git', 'clone', '--filter=blob:none', '--branch=stable', lazyrepo, lazypath }
  if vim.v.shell_error ~= 0 then error('Error cloning lazy.nvim:\n' .. out) end
end

---@type vim.Option
local rtp = vim.opt.rtp
rtp:prepend(lazypath)

require('lazy').setup({
  { 'NMAC427/guess-indent.nvim', opts = {} },

  {
    'folke/which-key.nvim',
    event = 'VimEnter',
    opts = {
      -- delay between pressing a key and opening which-key (milliseconds)
      delay = 0,
      icons = { mappings = vim.g.have_nerd_font },

      -- Document existing key chains
      spec = {
        { '<leader>b', group = '[B]uffer' },
        { '<leader>c', group = '[C]opy' },
        { '<leader>d', group = '[D]ebug' },
        { '<leader>e', group = '[E]xplorer' },
        { '<leader>g', group = '[G]it', mode = { 'n', 'v' } },
        { '<leader>m', group = '[M]arkdown' },
        { '<leader>s', group = '[S]earch', mode = { 'n', 'v' } },
        { '<leader>t', group = '[T]oggle' },
        { '<leader>w', group = '[W]indow' },
      },
    },
  },

  {
    'catppuccin/nvim',
    name = 'catppuccin',
    priority = 1000,
    opts = {
      flavour = 'frappe',
      no_italic = true,
    },
    config = function(_, opts)
      require('catppuccin').setup(opts)
      vim.cmd.colorscheme 'catppuccin'
    end,
  },

  {
    'nvim-mini/mini.nvim',
    config = function()
      require('mini.ai').setup { n_lines = 500 }
      require('mini.surround').setup()
      require('mini.pairs').setup()

      local statusline = require 'mini.statusline'
      statusline.setup { use_icons = vim.g.have_nerd_font }

      ---@diagnostic disable-next-line: duplicate-set-field
      statusline.section_location = function() return '%2l:%-2v' end
      ---@diagnostic disable-next-line: duplicate-set-field
      statusline.section_git = function() return '' end
      ---@diagnostic disable-next-line: duplicate-set-field
      statusline.section_diff = function() return '' end
      ---@diagnostic disable-next-line: duplicate-set-field
      statusline.section_fileinfo = function() return vim.bo.filetype end

      -- Always show path relative to cwd, even when LSP opens files with absolute paths
      ---@diagnostic disable-next-line: duplicate-set-field
      statusline.section_filename = function()
        if vim.bo.buftype == 'terminal' then return '%t' end
        local filepath = vim.fn.expand '%:p'
        local cwd = vim.fn.getcwd() .. '/'
        local rel = filepath:sub(1, #cwd) == cwd and filepath:sub(#cwd + 1) or filepath
        local flags = (vim.bo.modified and ' [+]' or '') .. (vim.bo.readonly and ' [RO]' or '')
        return rel .. flags
      end
    end,
  },

  {
    'nvim-treesitter/nvim-treesitter',
    config = function()
      local parsers = {
        'bash',
        'c',
        'css',
        'diff',
        'go',
        'gomod',
        'gosum',
        'html',
        'javascript',
        'json',
        'lua',
        'luadoc',
        'markdown',
        'markdown_inline',
        'python',
        'query',
        'ruby',
        'tsx',
        'typescript',
        'vim',
        'vimdoc',
        'yaml',
      }
      require('nvim-treesitter').install(parsers)
      -- Filetype names differ from parser names for some languages
      local filetypes = {
        'bash',
        'c',
        'css',
        'diff',
        'go',
        'gomod',
        'gosum',
        'html',
        'javascript',
        'json',
        'lua',
        'luadoc',
        'markdown',
        'markdown_inline',
        'python',
        'query',
        'ruby',
        'javascriptreact',
        'typescriptreact',
        'typescript',
        'vim',
        'vimdoc',
        'yaml',
      }
      vim.api.nvim_create_autocmd('FileType', {
        pattern = filetypes,
        callback = function() vim.treesitter.start() end,
      })
    end,
  },

  require 'kickstart.plugins.debug',
  -- require 'kickstart.plugins.indent_line', -- replaced by snacks.indent
  require 'kickstart.plugins.lint',
  -- require 'kickstart.plugins.autopairs', -- replaced by mini.pairs
  -- require 'kickstart.plugins.neo-tree', -- replaced by snacks.explorer
  require 'kickstart.plugins.gitsigns', -- adds gitsigns recommend keymaps

  { import = 'custom.plugins' },
}, {
  ui = {
    icons = vim.g.have_nerd_font and {} or {
      cmd = '⌘',
      config = '🛠',
      event = '📅',
      ft = '📂',
      init = '⚙',
      keys = '🗝',
      plugin = '🔌',
      runtime = '💻',
      require = '🌙',
      source = '📄',
      start = '🚀',
      task = '📌',
      lazy = '💤 ',
    },
  },
})
