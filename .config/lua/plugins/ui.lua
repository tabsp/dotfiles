return {
  {
    "folke/noice.nvim",
    event = "VeryLazy",
    opts = {},
    dependencies = {
      "MunifTanjim/nui.nvim",
      "rcarriga/nvim-notify",
    },
  },
  {
    "nvim-lualine/lualine.nvim",
    dependencies = { "nvim-tree/nvim-web-devicons" },
    config = function()
      require "plugins.lualine.evil_lualine"
      require("lualine").setup {
        options = {
          disabled_filetypes = {
            statusline = {
              "neo-tree",
            },
          },
        },
      }
    end,
  },
  {
    "shellRaining/hlchunk.nvim",
    event = { "BufReadPre", "BufNewFile" },
    config = function()
      local default_conf = {
        enable = true,
        style = "#806d9c",
        notify = false,
        priority = 0,
        exclude_filetypes = {
          neotree = true,
        },
      }
      require("hlchunk").setup {
        chunk = default_conf,
        line_num = default_conf,
        indent = default_conf,
        blank = {
          enable = true,
          chars = {
            " ",
          },
        },
      }
    end,
  },
  {
    "sphamba/smear-cursor.nvim",
    opts = {},
  },
}
