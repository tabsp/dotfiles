return {
  "nvim-lua/plenary.nvim",
  "MunifTanjim/nui.nvim",
  {
    "folke/tokyonight.nvim",
    lazy = false,
    opts = {
      style = "storm",
    },
    config = function(_, opts)
      local tokyonight = require "tokyonight"
      tokyonight.setup(opts)
      tokyonight.load()
    end,
  },
  { "nacro90/numb.nvim", event = "BufReadPre", config = true },
  {
    "famiu/bufdelete.nvim",
    cmd = { "Bdelete", "Bwipeout" },
  },
}
