local o = vim.opt
local nvim_cache_dir = vim.fn.stdpath('cache')

o.number = true
o.relativenumber = true
o.cursorline = true
o.wrap = true
o.showcmd = true
o.encoding = 'utf-8'
o.backup = false
o.swapfile = false
o.undofile = true
o.undodir = nvim_cache_dir .. '/undo'
o.clipboard = { 'unnamedplus' }
o.autochdir = true
o.incsearch = true
o.ignorecase = true
o.smartcase = true
o.termguicolors = true
o.list = true

