local packer = nil
local function init()
  if packer == nil then
    packer = require 'packer'
    packer.init { disable_commands = true }
  end

  local use = packer.use
  packer.reset()

  use 'wbthomason/packer.nvim'
  use { 'kyazdani42/nvim-tree.lua', requires = 'kyazdani42/nvim-web-devicons' }
  use { 'akinsho/nvim-bufferline.lua', requires = 'kyazdani42/nvim-web-devicons' }
  use { 'nvim-telescope/telescope.nvim', requires = { { 'nvim-lua/popup.nvim' }, { 'nvim-lua/plenary.nvim' } } }
  use { 'nvim-treesitter/nvim-treesitter', run = ':TSUpdate' }
  use { 'nvim-treesitter/nvim-treesitter-textobjects' }
  use { 'lewis6991/gitsigns.nvim', requires = { 'nvim-lua/plenary.nvim' } }
  use 'lukas-reineke/indent-blankline.nvim'
  use 'marko-cerovac/material.nvim'
  use { 'nvim-lualine/lualine.nvim', requires = { 'kyazdani42/nvim-web-devicons', opt = true } }
  use 'b3nj5m1n/kommentary'
  use 'Raimondi/delimitMate'
  use 'rhysd/accelerated-jk'
  use 'farmergreg/vim-lastplace'
  use { 'iamcco/markdown-preview.nvim', run = function() vim.fn['mkdp#util#install']() end, ft = { 'markdown', 'md' } }
  use 'neovim/nvim-lspconfig'
  use 'hrsh7th/nvim-cmp'
  use 'hrsh7th/cmp-nvim-lsp'
  use 'hrsh7th/cmp-buffer'
  use 'hrsh7th/cmp-path'
  use 'saadparwaiz1/cmp_luasnip'
  use 'L3MON4D3/LuaSnip'
  use 'famiu/bufdelete.nvim'
  use { 'ahmedkhalf/project.nvim', config = function () require("project_nvim").setup { detection_methods = { "lsp", "pattern" }, patterns = { ".git", "_darcs", ".hg", ".bzr", ".svn", "Makefile", "package.json" } } end }
end

local plugins = setmetatable({}, {
  __index = function(_, key)
    init()
    return packer[key]
  end,
})

return plugins
