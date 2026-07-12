local markdownlint_config = vim.fn.expand("~/.markdownlint-cli2.yaml")

return {
  {
    "stevearc/conform.nvim",
    optional = true,
    opts = {
      formatters = {
        ["markdownlint-cli2"] = {
          prepend_args = { "--config", markdownlint_config },
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
          prepend_args = { "--config", markdownlint_config },
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
