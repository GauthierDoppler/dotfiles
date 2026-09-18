return {
  {
    'mfussenegger/nvim-lint',
    event = { 'BufReadPre', 'BufNewFile' },
    config = function()
      local lint = require 'lint'
      lint.linters_by_ft = {
        kotlin = { 'ktlint' },
      }

      -- nvim-lint's bundled ktlint adapter parses stderr as JSON, but ktlint 1.8
      -- writes its INFO banner ("Enable default patterns [**/*.kt, **/*.kts]") to
      -- stderr *before* the report, so vim.json.decode throws and no diagnostic
      -- ever surfaces. --log-level=none silences the banner and leaves clean JSON.
      local ktlint = lint.linters.ktlint
      ktlint.args = { '--reporter=json', '--log-level=none', '--stdin' }

      -- ktlint is a JVM process (~0.4-0.8s per run), so it is only worth spawning
      -- on read and on write -- not on InsertLeave, which would fire a JVM every
      -- time you leave insert mode.
      vim.api.nvim_create_autocmd({ 'BufReadPost', 'BufWritePost' }, {
        group = vim.api.nvim_create_augroup('lint', { clear = true }),
        callback = function()
          if vim.bo.modifiable then lint.try_lint() end
        end,
      })
    end,
  },
}
