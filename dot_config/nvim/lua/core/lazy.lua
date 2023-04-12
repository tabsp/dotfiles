local fn = vim.fn
local install_path = fn.stdpath "data" .. "/lazy/lazy.nvim"

if not vim.loop.fs_stat(install_path) then
  print "Cloning lazy ..."
  fn.system {
    "git",
    "clone",
    "--filter=blob:none",
    "https://github.com/folke/lazy.nvim.git",
    "--branch=stable",
    install_path,
  }
end
vim.opt.rtp:prepend(install_path)

require("lazy").setup {
  spec = {
    { import = "plugins" },
  },
  install = { colorscheme = { "tokyonight" } },
  defaults = { lazy = true },
  performance = {
    rtp = {
      disabled_plugins = { "tohtml", "gzip", "matchit", "zipPlugin", "netrwPlugin", "tarPlugin" },
    },
  },
}
