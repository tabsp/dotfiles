return {
  -- Disable spell checking for markdown (LazyVim enables it by default)
  {
    "LazyVim/LazyVim",
    optional = true,
    init = function()
      vim.api.nvim_create_autocmd("FileType", {
        group = vim.api.nvim_create_augroup("user_markdown", { clear = true }),
        pattern = "markdown",
        callback = function()
          vim.opt_local.spell = false
        end,
      })
    end,
  },
  {
    "stevearc/conform.nvim",
    optional = true,
    opts = {
      formatters = {
        ["markdownlint-cli2"] = {
          prepend_args = { "--config", vim.fn.expand("~/.markdownlint-cli2.yaml") },
        },
      },
    },
  },
  {
    "mfussenegger/nvim-lint",
    optional = true,
    opts = {
      linters = {
        ["markdownlint-cli2"] = {
          args = { "--config", vim.fn.expand("~/.markdownlint-cli2.yaml"), "-" },
        },
      },
    },
  },
  {
    "MeanderingProgrammer/render-markdown.nvim",
    dependencies = {
      "nvim-treesitter/nvim-treesitter",
      "nvim-mini/mini.icons",
    },
    ft = { "markdown", "markdown.mdx", "norg", "rmd", "org", "codecompanion" },
    opts = {
      heading = {
        enabled = true,
        sign = true,
        style = "full",
        icons = { "1. ", "2. ", "3. ", "4. ", "5. ", "6. " },
        left_pad = 1,
      },
      bullet = {
        enabled = true,
        icons = { "*", "-", "+", "-" },
        right_pad = 1,
        highlight = "RenderMarkdownBullet",
      },
      checkbox = {
        enabled = true,
        unchecked = {
          icon = "[ ] ",
          highlight = "RenderMarkdownUnchecked",
        },
        checked = {
          icon = "[x] ",
          highlight = "RenderMarkdownChecked",
        },
        custom = {
          todo = {
            raw = "[-]",
            rendered = "[-] ",
            highlight = "RenderMarkdownTodo",
          },
        },
      },
    },
  },
}
