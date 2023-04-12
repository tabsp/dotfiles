return {
  {
    "akinsho/nvim-bufferline.lua",
    event = "VeryLazy",
    opts = {
      options = {
        mode = "buffers", -- tabs or buffers
      },
    },
  },
  -- scope buffers to tabs
  { "tiagovla/scope.nvim", event = "VeryLazy", opts = {} },
}
