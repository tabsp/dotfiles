vim.cmd "packadd packer.nvim"

local plugins = {
  ["wbthomason/packer.nvim"] = {},
  ["nvim-lua/plenary.nvim"] = { module = "plenary" },
  ["kyazdani42/nvim-web-devicons"] = {},
  ["kyazdani42/nvim-tree.lua"] = {
    ft = "alpha",
    cmd = { "NvimTreeToggle", "NvimTreeFocus" },
    config = function()
      require "plugins.configs.nvimtree"
    end,
  },
  ["nvim-treesitter/nvim-treesitter"] = {
    module = "nvim-treesitter",
    setup = function()
      require("core.lazy_load").on_file_open "nvim-treesitter"
    end,
    cmd = require("core.lazy_load").treesitter_cmds,
    run = ":TSUpdate",
    config = function()
      require "plugins.configs.treesitter"
    end,
  },
  ["lukas-reineke/indent-blankline.nvim"] = {
    opt = true,
    setup = function()
      require("core.lazy_load").on_file_open "indent-blankline.nvim"
    end,
    config = function()
      require("plugins.configs.others").blankline()
    end,
  },
  ["folke/tokyonight.nvim"] = {
    config = require("plugins.configs.others").tokyonight(),
  },
  -- lsp plugins
  ["williamboman/nvim-lsp-installer"] = {
    opt = true,
    cmd = require("core.lazy_load").lsp_cmds,
    setup = function()
      require("core.lazy_load").on_file_open "nvim-lsp-installer"
    end,
  },
  ["neovim/nvim-lspconfig"] = {
    after = "nvim-lsp-installer",
    module = "lspconfig",
    config = function()
      require "lsp"
    end,
  },

  ["jose-elias-alvarez/null-ls.nvim"] = {
    after = "nvim-lspconfig",
    config = function()
      require "plugins.configs.null-ls"
    end,
  },

  -- load luasnips + cmp related in insert mode only
  ["rafamadriz/friendly-snippets"] = {
    module = "cmp_nvim_lsp",
    event = "InsertEnter",
  },
  ["hrsh7th/nvim-cmp"] = {
    after = "friendly-snippets",
    config = function()
      require "plugins.configs.cmp"
    end,
  },
  ["L3MON4D3/LuaSnip"] = {
    wants = "friendly-snippets",
    after = "nvim-cmp",
    config = function()
      require("plugins.configs.others").luasnip()
    end,
  },
  ["saadparwaiz1/cmp_luasnip"] = {
    after = "LuaSnip",
  },
  ["hrsh7th/cmp-nvim-lua"] = {
    after = "cmp_luasnip",
  },
  ["hrsh7th/cmp-nvim-lsp"] = {
    after = "cmp-nvim-lua",
  },
  ["hrsh7th/cmp-buffer"] = {
    after = "cmp-nvim-lsp",
  },
  ["hrsh7th/cmp-path"] = {
    after = "cmp-buffer",
  },

  ["nvim-lualine/lualine.nvim"] = {
    config = function()
      require "ui.eviline"
    end,
  },
  ["akinsho/bufferline.nvim"] = {
    tag = "v2.*",
    setup = function()
      require("core.lazy_load").on_file_open "bufferline.nvim"
    end,
    config = function()
      require("plugins.configs.others").bufferline()
    end,
  },
  ["numToStr/Comment.nvim"] = {
    config = function()
      require("plugins.configs.others").comment()
    end,
  },
  ["windwp/nvim-autopairs"] = {
    after = "nvim-cmp",
    config = function()
      require("plugins.configs.others").autopairs()
    end,
  },
  ["lewis6991/gitsigns.nvim"] = {
    opt = true,
    setup = function()
      require("core.lazy_load").gitsigns()
    end,
    config = function()
      require("plugins.configs.others").gitsigns()
    end,
  },
  ["famiu/bufdelete.nvim"] = {},

  ["nvim-telescope/telescope.nvim"] = {
    cmd = "Telescope",
    config = function()
      require "plugins.configs.telescope"
    end,
  },

  ["ethanholz/nvim-lastplace"] = {
    config = function()
      require("plugins.configs.others").lastplace()
    end,
  },

  ["ahmedkhalf/project.nvim"] = {
    config = function()
      require("plugins.configs.others").project()
    end,
  },
}

require("core.packer").run(plugins)
