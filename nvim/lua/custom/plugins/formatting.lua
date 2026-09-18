return {
  { -- Autoformat
    'stevearc/conform.nvim',
    event = { 'BufWritePre' },
    cmd = { 'ConformInfo' },
    keys = {
      {
        '<leader>f',
        function() require('conform').format { async = true, lsp_format = 'fallback' } end,
        mode = '',
        desc = '[F]ormat buffer',
      },
    },
    opts = {
      notify_on_error = false,
      format_on_save = function(bufnr)
        -- Disable "format_on_save lsp_fallback" for languages that don't
        -- have a well standardized coding style. You can add additional
        -- languages here or re-enable it for the disabled ones.
        local disable_filetypes = { c = true, cpp = true }
        if disable_filetypes[vim.bo[bufnr].filetype] then return nil end

        -- ktlint is a JVM process: ~0.4s on a small file, ~0.8s on a 4k-line one.
        -- At the default 500ms it silently times out and the buffer saves
        -- unformatted (notify_on_error = false hides it), which then fails
        -- `./gradlew ktlintCheck` in CI. Same for swift-format, which goes
        -- through `xcrun` and pays that lookup on every run.
        local slow_filetypes = { kotlin = true, swift = true }
        return {
          timeout_ms = slow_filetypes[vim.bo[bufnr].filetype] and 3000 or 500,
          lsp_format = 'fallback',
        }
      end,
      formatters_by_ft = {
        lua = { 'stylua' },
        typescript = { 'biome' },
        typescriptreact = { 'biome' },
        javascript = { 'biome' },
        javascriptreact = { 'biome' },
        json = { 'biome' },
        go = { 'goimports' },
        ruby = { 'rubocop' },
        python = { 'ruff_format', 'ruff_organize_imports' },
        kotlin = { 'ktlint' },
        swift = { 'swift_format' },
      },
      formatters = {
        -- swift-format ships inside Xcode's toolchain and is not on PATH (unlike
        -- sourcekit-lsp, which is shimmed into /usr/bin). xcrun is the only thing
        -- that knows where it is for the selected Xcode.
        swift_format = { command = 'xcrun', prepend_args = { 'swift-format' } },
      },
    },
  },
}
