require("lazy").setup({
  spec = {
    { "LazyVim/LazyVim", import = "lazyvim.plugins" },
    { import = "plugins" },
  },
  install = { colorscheme = { "catppuccin", "tokyonight", "habamax" } },
  ui = { border = "rounded" },
  checker = { enabled = true },
})
