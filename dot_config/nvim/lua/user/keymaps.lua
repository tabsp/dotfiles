vim.g.mapleader = ' '

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
nnoremap '<C-d> :Bdelete<CR>'

nnoremap '<C-k> <C-w>k'
nnoremap '<C-j> <C-w>j'
nnoremap '<C-h> <C-w>h'
nnoremap '<C-l> <C-w>l'

nnoremap '<leader>lg :<C-u>tabe<CR>:-tabmove<CR>:term lazygit<CR>'


-- rhysd/accelerated-jk
nmap 'j <Plug>(accelerated_jk_gj)'
nmap 'k <Plug>(accelerated_jk_gk)'

-- kyazdani42/nvim-tree.lua
nnoremap 'tt :NvimTreeToggle<CR>'

-- akinsho/nvim-bufferline.lua
nnoremap '<leader>= :BufferLineCycleNext<CR>'
nnoremap '<leader>- :BufferLineCyclePrev<CR>'

-- nvim-telescope/telescope.nvim
nnoremap '<leader>ff <cmd>lua require("telescope.builtin").find_files(require("telescope.themes").get_dropdown({}))<CR>'
nnoremap '<leader>fg <cmd>lua require("telescope.builtin").live_grep(require("telescope.themes").get_dropdown({}))<CR>'
nnoremap '<leader>fb <cmd>lua require("telescope.builtin").buffers(require("telescope.themes").get_dropdown({}))<CR>'
nnoremap '<leader>fh <cmd>lua require("telescope.builtin").help_tags(require("telescope.themes").get_dropdown({}))<CR>'
