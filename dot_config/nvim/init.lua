local o = vim.opt
local g = vim.g
local fn = vim.fn
local execute = vim.api.nvim_command

local nvim_data_dir = fn.stdpath('data')
local nvim_cache_dir = fn.stdpath('cache')

local os_name = vim.loop.os_uname().sysname
local is_windows = os_name == 'Windows'
local path_sep = is_windows and '\\' or '/'

o.number = true
o.relativenumber = true
o.cursorline = true
o.wrap = true
o.showcmd = true
o.encoding = 'utf-8'
o.backup = false
o.swapfile = false
o.undofile = true
o.undodir = nvim_cache_dir .. path_sep .. 'undo'
o.clipboard = { 'unnamedplus' }
o.autochdir = true
o.incsearch = true
o.ignorecase = true
o.smartcase = true
o.termguicolors = true
o.list = true

g.mapleader = ' '

-- Highlight on yank
vim.cmd [[
  augroup YankHighlight
    autocmd!
    autocmd TextYankPost * silent! lua vim.highlight.on_yank()
  augroup end
]]

local function noremap(mode, binding)
  local opts = {
    noremap = true,
    silent = true,
    expr = false,
    nowait = false,
  }
  local lhs, rhs = binding:match('([^%s]*)%s?(.+)')
  vim.api.nvim_set_keymap(mode, lhs, rhs, opts)
end

local function nnoremap(binding)
  noremap('n', binding)
end

local function inoremap(binding)
  noremap('i', binding)
end

local function xnoremap(binding)
  noremap('x', binding)
end

local function map(mode, binding)
  local opts = {
    noremap = false,
    silent = true,
    expr = false,
    nowait = false,
  }
  local lhs, rhs = binding:match('([^%s]*)%s?(.+)')
  vim.api.nvim_set_keymap(mode, lhs, rhs, opts)
end

local function nmap(binding)
  map('n', binding)
end

nnoremap '<leader><CR> :set hlsearch!<CR>'
nnoremap '<leader>rc :e $MYVIMRC<CR>'
xnoremap '< gv'
xnoremap '> >gv'
inoremap 'jj <ESC>'
nnoremap 'H ^'
nnoremap 'L $'
nnoremap 'S :w<CR>'
nnoremap 'Q :q<CR>'
nnoremap '<C-d> :bdelete<CR>'

nnoremap '<C-k> <C-w>k'
nnoremap '<C-j> <C-w>j'
nnoremap '<C-h> <C-w>h'
nnoremap '<C-l> <C-w>l'

nnoremap '<leader>lg :<C-u>tabe<CR>:-tabmove<CR>:term lazygit<CR>'

-- wbthomason/packer.nvim
local packer_dir = nvim_data_dir .. path_sep .. 'site' .. path_sep .. 'pack' .. path_sep .. 'packer' .. path_sep .. 'start' .. path_sep .. 'packer.nvim'
if fn.empty(fn.glob(packer_dir)) > 0 then
  vim.notify('Downloading packer.nvim ...')
  vim.notify(
    fn.system { 'git', 'clone', 'https://github.com/wbthomason/packer.nvim', packer_dir }
  )
  require("plugins").install()
end

execute 'command! PackerInstall packadd packer.nvim | lua require("plugins").install()'
execute 'command! PackerUpdate packadd packer.nvim | lua require("plugins").update()'
execute 'command! PackerSync packadd packer.nvim | lua require("plugins").sync()'
execute 'command! PackerClean packadd packer.nvim | lua require("plugins").clean()'
execute 'command! PackerCompile packadd packer.nvim | lua require("plugins").compile()'

-- neovim/nvim-lspconfig
require('mylsp')

-- marko-cerovac/material.nvim
g.material_style = "darker"
vim.cmd 'colorscheme material'

-- nvim-lualine/lualine.nvim
require('eviline')

-- kyazdani42/nvim-tree.lua
require('nvim-tree').setup{}
nnoremap 'tt :NvimTreeToggle<CR>'


-- akinsho/nvim-bufferline.lua
require('bufferline').setup{}
nnoremap '<leader>= :BufferLineCycleNext<CR>'
nnoremap '<leader>- :BufferLineCyclePrev<CR>'

-- nvim-telescope/telescope.nvim
nnoremap '<leader>ff <cmd>lua require("telescope.builtin").find_files(require("telescope.themes").get_dropdown({}))<CR>'
nnoremap '<leader>fg <cmd>lua require("telescope.builtin").live_grep(require("telescope.themes").get_dropdown({}))<CR>'
nnoremap '<leader>fb <cmd>lua require("telescope.builtin").buffers(require("telescope.themes").get_dropdown({}))<CR>'
nnoremap '<leader>fh <cmd>lua require("telescope.builtin").help_tags(require("telescope.themes").get_dropdown({}))<CR>'

-- nvim-treesitter/nvim-treesitter
require('nvim-treesitter.configs').setup {
  ensure_installed = "maintained",
  highlight = {
    enable = true,
  },
  textobjects = {
    select = {
      enable = true,
      lookahead = true,
      keymaps = {
        ['af'] = '@function.outer',
        ['if'] = '@function.inner',
        ['ac'] = '@class.outer',
        ['ic'] = '@class.inner',
      },
    },
    move = {
      enable = true,
      set_jumps = true,
      goto_next_start = {
        [']m'] = '@function.outer',
        [']]'] = '@class.outer',
      },
      goto_next_end = {
        [']M'] = '@function.outer',
        [']['] = '@class.outer',
      },
      goto_previous_start = {
        ['[m'] = '@function.outer',
        ['[['] = '@class.outer',
      },
      goto_previous_end = {
        ['[M'] = '@function.outer',
        ['[]'] = '@class.outer',
      },
    },
  }
}

-- lewis6991/gitsigns.nvim
require('gitsigns').setup{}

-- lukas-reineke/indent-blankline.nvim
require("indent_blankline").setup {
  show_current_context = true,
  show_current_context_start = true,
}

-- b3nj5m1n/kommentary
require('kommentary.config').use_extended_mappings()

-- Raimondi/delimitMate
g.delimitMate_expand_cr = 0
g.delimitMate_expand_space = 1
g.delimitMate_smart_quotes = 1
g.delimitMate_expand_inside_quotes = 0

-- rhysd/accelerated-jk
nmap 'j <Plug>(accelerated_jk_gj)'
nmap 'k <Plug>(accelerated_jk_gk)'

-- farmergreg/vim-lastplace
g.lastplace_ignore = 'gitcommit,gitrebase,svn,hgcommit'
g.lastplace_ignore_buftype = 'quickfix,nofile,help'

-- iamcco/markdown-preview.nvim
g.mkdp_auto_start = 0
g.mkdp_auto_close = 1
