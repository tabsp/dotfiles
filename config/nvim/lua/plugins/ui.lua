return {
  {
    "folke/noice.nvim",
    event = "VeryLazy",
    opts = {
      lsp = {
        override = {
          ["vim.lsp.util.convert_input_to_markdown_lines"] = true,
          ["vim.lsp.util.stylize_markdown"] = true,
          ["cmp.entry.get_documentation"] = true,
        },
      },
    },
    dependencies = {
      "MunifTanjim/nui.nvim",
      { "rcarriga/nvim-notify", opts = {} },
    },
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
  {
    "lewis6991/gitsigns.nvim",
    opts = {
      signs = {
        add = { text = "+" },
        change = { text = "~" },
        delete = { text = "_" },
        topdelete = { text = "‾" },
        changedelete = { text = "~" },
      },
    },
  },
  { -- Collection of various small independent plugins/modules
    "echasnovski/mini.nvim",
    config = function()
      -- Better Around/Inside textobjects
      --
      -- Examples:
      --  - va)  - [V]isually select [A]round [)]paren
      --  - yinq - [Y]ank [I]nside [N]ext [Q]uote
      --  - ci'  - [C]hange [I]nside [']quote
      require("mini.ai").setup { n_lines = 500 }

      -- Add/delete/replace surroundings (brackets, quotes, etc.)
      --
      -- - saiw) - [S]urround [A]dd [I]nner [W]ord [)]Paren
      -- - sd'   - [S]urround [D]elete [']quotes
      -- - sr)'  - [S]urround [R]eplace [)] [']
      require("mini.surround").setup()
    end,
  },
  {
    "nvim-lualine/lualine.nvim",
    dependencies = { "nvim-tree/nvim-web-devicons" },
    event = "VeryLazy",
    opts = {
      options = {
        theme = "catppuccin-nvim",
        globalstatus = true,
      },
      sections = {
        lualine_c = {
          {
            "filename",
            path = 1,
          },
        },
        lualine_x = { "encoding", "fileformat", "filetype" },
      },
    },
  },
  {
    "MeanderingProgrammer/render-markdown.nvim",
    dependencies = { "nvim-treesitter/nvim-treesitter", "echasnovski/mini.nvim" },
    opts = {
      latex = { enabled = false },
    },
  },
}
