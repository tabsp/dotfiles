return {
  {
    "nvim-treesitter/nvim-treesitter",
    build = ":TSUpdate",
    config = function()
      local treesitter = require "nvim-treesitter"
      local languages = { "bash", "c", "html", "lua", "markdown", "markdown_inline", "regex", "vim", "yaml" }

      treesitter.setup()

      local function install_parsers()
        if vim.fn.executable "tree-sitter" == 1 then
          local task = treesitter.install(languages)
          if #vim.api.nvim_list_uis() == 0 then
            task:wait(300000)
          end
        end
      end

      install_parsers()

      vim.api.nvim_create_autocmd("User", {
        pattern = "MasonToolsUpdateCompleted",
        callback = install_parsers,
      })

      vim.api.nvim_create_autocmd("FileType", {
        pattern = languages,
        callback = function()
          pcall(vim.treesitter.start)
          vim.bo.indentexpr = "v:lua.require'nvim-treesitter'.indentexpr()"
        end,
      })
    end,
  },
}
