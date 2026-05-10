local o = vim.opt
local g = vim.g

o.laststatus = 2 -- global statusline
o.showmode = false

o.title = true
o.clipboard = "unnamedplus"
o.cul = true -- cursor line

-- Indenting
o.expandtab = true
o.shiftwidth = 2
o.smartindent = true

o.fillchars = { eob = " " }
o.incsearch = true
o.ignorecase = true
o.smartcase = true
o.mouse = "a"

-- numbers
o.number = true
o.numberwidth = 2
o.relativenumber = true
o.ruler = false

-- disable nvim intro
o.shortmess:append "sI"

o.signcolumn = "yes"
o.splitbelow = true
o.splitright = true
o.tabstop = 8
o.termguicolors = true
o.timeoutlen = 400
o.undofile = true

o.showtabline = 2

-- interval for writing swap file to disk, also used by gitsigns
o.updatetime = 250

-- go to previous/next line with h,l,left arrow and right arrow
-- when cursor reaches end/beginning of line
o.whichwrap:append "<>[]hl"

g.mapleader = " "
g.maplocalleader = "\\"
