return {
  {
    'r-pletnev/pdfreader.nvim',
    lazy = false,
    dependencies = {
      'folke/snacks.nvim',
    },
    config = function()
      require('pdfreader').setup()
    end,
  },
}
