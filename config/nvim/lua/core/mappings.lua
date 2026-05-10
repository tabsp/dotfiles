-- n, v, i, t = mode names
local M = {}

M.general = {

  i = {
    -- go to  beginning and end
    ["<C-a>"] = { "<ESC>^i", "beginning of line" },
    ["<C-e>"] = { "<End>", "end of line" },

    -- navigate within insert mode
    ["<C-h>"] = { "<Left>", "move left" },
    ["<C-l>"] = { "<Right>", "move right" },
    ["<C-j>"] = { "<Down>", "move down" },
    ["<C-k>"] = { "<Up>", "move up" },

    ["jj"] = { "<ESC>", "esc" },
  },

  n = {

    ["<leader>rc"] = { ":e $MYVIMRC<CR>", "open init.lua" },
    ["<ESC>"] = { "<cmd> noh <CR>", "no highlight" },

    -- switch between windows
    ["<C-h>"] = { "<C-w>h", "window left" },
    ["<C-l>"] = { "<C-w>l", "window right" },
    ["<C-j>"] = { "<C-w>j", "window down" },
    ["<C-k>"] = { "<C-w>k", "window up" },

    -- save
    ["<leader>w"] = { "<cmd> w <CR>", "save file" },

    -- Copy all
    ["<C-c>"] = { "<cmd> %y+ <CR>", "copy whole file" },

    ["Q"] = { "<cmd> q <CR>", "Quit" },

    ["H"] = { "^", "beginning of line" },
    ["L"] = { "$", "end of line" },
  },
}

M.neotree = {
  n = {
    ["tt"] = { "<cmd> Neotree toggle reveal <CR>", "toggle neotree" },
  },
}

M.bufdelete = {
  n = {
    ["<C-d>"] = { "<cmd> Bdelete <CR>", "delete buffer" },
  },
}

M.tabby = {
  n = {
    ["<leader>tn"] = { "<cmd> tabn <CR>", "next tab" },
    ["<leader>="] = { "<cmd> tabn <CR>", "next tab" },
    ["<leader>tp"] = { "<cmd> tabp <CR>", "previous tab" },
    ["<leader>-"] = { "<cmd> tabp <CR>", "previous tab" },
    ["<leader>tc"] = { "<cmd> tabclose <CR>", "close tab" },
    ["<leader>ta"] = { "<cmd> tabnew <CR>", "new tab" },
    ["<leader>to"] = { "<cmd> tabonly <CR>", "only tab" },
    ["<leader>tmp"] = { "<cmd> -tabmove <CR>", "move current tab to previous position" },
    ["<leader>tmn"] = { "<cmd> +tabmove <CR>", "move current tab to next position" },
  },
}

return M
